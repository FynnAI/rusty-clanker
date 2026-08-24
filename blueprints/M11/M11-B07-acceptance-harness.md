# M11-B07 — M11 Acceptance Harness & Roadmap Completion Report

| Field | Content |
|---|---|
| ID | M11-B07 |
| Milestone | M11 — Bedrock Cross-Play |
| Prerequisites | **M11-B01** (`rc-bedrock-raknet`) — read in full; this blueprint's own `BedrockBot` (Context §D) is a client-side driver over `RaknetListener::accept`'s counterpart, `RaknetSession::{send, recv}` (M11-B01 §L), and reuses `Reliability`/`OrderChannel` unmodified. **M11-B02** (`rc-bedrock-protocol`) — read in full; `BedrockBot` constructs and decodes the identical M11 packet catalog (`RequestNetworkSettingsPacket` through `StartGamePacket`, `PlayerAuthInputPacket`, `TextPacket`) via `BedrockPacket::encode`/`decode`, `encode_batch`/`decode_batch`, `pack_sub_packet`/`unpack_sub_packet`, unmodified. **M11-B03** (`rc-bedrock-auth`) — read in full; `BedrockBot` reuses `handshake::{ServerEcdhKeyPair, SharedSecret, BedrockAeadEncryptor, BedrockAeadDecryptor, generate_salt}` **directly, unmodified** for its own client-side ECDH/AES-GCM math (Context §D explains precisely why this is sound reuse, not a misuse of a server-named type), and restates only the one asymmetric piece that crate never exposes — client-side JWT chain *signing* (Context §D.1). **M11-B04** (`rc-bedrock-mappings`) — cited only for confidence that a synthetic `BedrockBlockState`/`BedrockBiomeId` fixture (never `MappingTables::load()` against real generated data, mirroring that blueprint's own established test-fixture discipline) is the correct shape for this blueprint's own tier-conformance cross-check (§I). **M11-B05** (`rc-bedrock-translator`) — read in full, **the hard prerequisite this blueprint's own Done state depends on most**: `login::{step, LoginPhase, LoginEvent}`, `TIER_TABLE`/`TierEntry`/`Tier` (Context §N), and every `translate_*` function's own already-real, already-Tier-1-green test suite (`session_state_machine.rs`, `chunk_translation.rs`, `entity_metadata_golden_pairs.rs`, `inventory_transaction_matrix.rs`, `tier_table_conformance.rs`) are cited by name and reused as this blueprint's own evidence for "the translation logic itself is proven," never re-tested here. **M11-B06** (`rusty-clanker-server`/`rc-proxy` activation) — read in full, the blueprint this one extends most directly: `composition::bedrock::{run_bedrock_login, BedrockLoginOutcome, BedrockLoginError, BedrockTranslator, BedrockSessionHandoff, UnavailableBedrockTranslator, ServerMotdProvider, DEFAULT_MOTD}`, `CrossplayConfig`/`ResourcePackConfig`, `ServerComposition::player_count`, and — load-bearing — that blueprint's own already-real `crossplay_disabled_binds_zero_bedrock_sockets_and_loads_zero_mapping_tables`/`crossplay_never_references_mapping_tables_load`/`monolithic_dual_listener_accepts_java_and_bedrock_simultaneously` tests, reused and extended rather than duplicated (Context §E/§F). `rc-proxy`'s own `bedrock::{run_bedrock_login, hand_off_to_node}`, `NodeAcceptor::try_recv_bedrock`, `Edition::Bedrock`, `ForwardedIdentity::xuid` (Context §J). **M7-B09** (Cluster Mode Acceptance Harness) — read in full, this blueprint's own direct structural template: its §A "scope boundary, gap named once" framing, its §H "AC4a runtime inertness + AC4b prior-milestone re-invocation" split, its `xtask::m7_report::{M7ReportError::ClusterIntegrationPending, M7CompletionReport}` shape, its `deploy/cluster/docker-compose.m7-acceptance.yml` overlay-file technique, and its own inherited-gap framing (`EXIT_CLUSTER_INTEGRATION_PENDING`, `M7-B08`'s still-open Context §A items 1/3) — all restated and extended here, never re-derived from scratch (Context §A/§L/§N). **M7-B08** (cluster bootstrap/config) — cited for `EXIT_CLUSTER_INTEGRATION_PENDING: i32 = 3` and `deploy/cluster/docker-compose.cluster-test.yml`'s own base topology, reused unmodified as this blueprint's own cluster-mode leg's base file (Context §J). **M10-B06** (Phase-2 completion report) — cited for its own `Phase2Gate`/`PHASE2_NOTE`/`read_m9_status` pattern, the direct precedent this blueprint's own Roadmap Completion rollup (Context §M) extends from a two-milestone gate to the full `M0`–`M11` sequence. **M6-B01** (`rc-paritybot`'s own load-test extension) and **M1-B06** (`rc-paritybot`'s own founding blueprint) — read for the crate's own established shape: `crates/testing/paritybot/` (crate `rc-paritybot`), its existing `azalea` dependency (TEST-D8), its existing `pub mod loadtest;` line in `src/lib.rs`, and — load-bearing — the already-committed `PROTECTED_PATHS` row `ProtectedPath { pattern: "crates/testing/paritybot/**", .. }` (M1-B06), which this blueprint's own new `src/bedrock_bot/` module and `tests/`/`scenarios/` additions fall under **without needing a new path-guard row** (Context §D, Constraints). **M5-B10** (worldgen corpus harness) — read for `xtask::worldgen_corpus::{hash_block_state_column, hash_biome_column}` over `rc_chunk_storage::column::{BlockStateColumn, BiomeColumn}` (its own §C), reused **unmodified** as the two of three layers this blueprint's own cross-edition world-state comparator needs (Context §H) — that blueprint's own doc comment explicitly anticipates this exact extension ("a future revision... may add a third, diagnostic-only... hash without changing this blueprint's own gate definition"). **M0-B08** (`xtask::tier_result::{TierResult, CaseResult, Status, write, write_to, VERIFY_OUT_DIR, exit_code_for}`, `xtask::path_guard`) — reused unmodified, the fixed template every `M<n>-report` in this corpus already follows. |
| Implements | `11-roadmap-milestones.md`'s M11 Acceptance Criteria, verbatim (Context §B) — this blueprint **is** their concrete, agent-executable measurement, per PLAN-D5, exactly as every prior `M<n>-B0x` acceptance-harness blueprint already is for its own milestone. CROSS-D22 (the M11 milestone text itself, restated). CROSS-D23 (the Bedrock-bot-driver decision — restated and, for the first time in this corpus, **implemented**, Context §C/§D). CROSS-D24 (the cross-edition consistency suite, extended per its own text from `09`'s scenario-corpus format, honestly scoped to what already exists in this corpus, Context §G/§H). CROSS-D25 (the one manual-verification carve-out, Context §K). CROSS-D26 (zero-cost-when-off — the acceptance-level proof, extending M11-B06's own compile/config-level proof with a runtime benchmark and a stronger runtime-inertness check, Context §E). CROSS-D1/D15–D18 (Java-semantics authority and the translation tier framework — the literal subject of Context §I's conformance suite). TEST-D8/D10/D21/D23/D24/D25 (restated in full against this milestone, Context §C/§G/§H/§J). TEST-D34/D37/D40/D43 (CI tier placement, machine-readable output, agent operability — restated §N). TEST-D45/D46/D50/D52 (test-first changeset boundary, CI-is-authority, independent verification — restated). PLAN-D5/D6 (milestone completion gated exclusively by measurable acceptance criteria; this blueprint's own Roadmap Completion rollup, Context §M, is the first machine-readable artifact in this corpus stating the `M0`–`M11` sequence has reached its own final node). |
| Crates touched | `crates/testing/paritybot/` (`rc-paritybot`) — additive: `Cargo.toml` (three new path dependencies, already-workspace-pinned crates, Context §D), `src/lib.rs` (`pub mod bedrock_bot;`), `src/bedrock_bot/{mod.rs, chain_signing.rs}` (new), `scenarios/crossplay/*.ron` (two new worked-example fixtures), `tests/bedrock_bot_*.rs` (new). `xtask` (additive: `src/m11_report.rs`, `src/lib.rs` (+1 line), `src/main.rs` (+1 `Command` variant), `tests/m11_report.rs`). `crates/server/benches/crossplay_zero_cost.rs` (new — a `criterion` benchmark, Context §E). `.github/workflows/ci.yml` (one new `workflow_dispatch`-only job, `m11-acceptance-gate`; `inputs.job`'s choice list gains `m11-acceptance`). `deploy/cluster/docker-compose.m11-acceptance.yml` (new, non-code, a compose **overlay**, never a modification of `M7-B08`'s or `M7-B09`'s own already-committed compose files). `docs/MANUAL-VERIFICATION-M11-B07.md` (new). **Not** any file under `crates/bedrock-raknet/`, `crates/bedrock-protocol/`, `crates/bedrock-auth/`, `crates/bedrock-mappings/`, `crates/bedrock-translator/`, `crates/server/src/`, or `crates/proxy/src/` — every Bedrock-side production mechanism this blueprint measures already exists, fully specified, in a prerequisite blueprint; this blueprint adds **zero** new production Bedrock behavior, only measurement, orchestration, and one new reusable test-driver crate module. |
| Estimated scope | L, explicitly and substantially beyond the nominal single-blueprint size class — the same deliberate, cited exception every acceptance-harness blueprint in this lineage already takes (`M6-B06`, `M7-B09`, `M8-B05`, `M9-B07`, `M10-B06`), here at its own extreme end: seven roadmap acceptance criteria, a from-scratch bot driver, a cross-edition comparator, an inherited two-part gap (the still-missing composition-root/translator wiring *and* the still-open `M7-B08` cluster-integration gap), and — uniquely among this corpus's harness blueprints, since M11 is the roadmap's own final node — a full-roadmap completion rollup, share enough plumbing (the `TierResult`/`M<n>ReportResult` template, the honest-gate discipline, the compose-overlay technique) that splitting them into separate blueprints would force each to restate the others' shared vocabulary from scratch while leaving no single blueprint that actually answers "is M11, and therefore the whole roadmap `M0`–`M11`, done." |

## Goal & Done definition

Wire every one of M11's seven roadmap acceptance criteria (`11-roadmap-milestones.md`, quoted verbatim §B) into one precise, agent-executable, machine-readable measurement, `xtask m11-report`, continuing the exact `M<n>ReportResult` lineage `M0-B08`→`M1-B06`→…→`M10-B06` already established — built entirely against real, already-committed `M11-B01`–`M11-B06` code, never a hand-built stub standing in for any of them. Concretely:

1. **The Bedrock-bot driver** (`rc-paritybot::bedrock_bot::BedrockBot`, Context §C/§D) — CROSS-D23's own decision, implemented for the first time: our own `rc-bedrock-raknet`/`rc-bedrock-protocol`/`rc-bedrock-auth` crates, driven client-side, added to `rc-paritybot` (`09`'s existing differential-testing crate, TEST-D1/D8), never a new third-party dependency, its own honest independence caveat restated and never glossed over.
2. **The join-and-play leg (AC2)** — a real `BedrockBot` completes the **full** connection lifecycle (RakNet handshake, JWT-chain login, ECDH/AES-GCM encryption handshake, resource-pack negotiation, §E's algorithm) against a real `rusty-clanker-server` — live, green, Tier-1, today — and then, honestly, receives exactly `UnavailableBedrockTranslator`'s own `DisconnectPacket`, never a `StartGame` (Context §F). "Spawns into the server's real, `M5`-generated terrain" is precisely, and only, what remains gated — on a still-missing composition-root/ECS-adapter *and* Stage-11-integration blueprint (named exactly as `M11-B05`/`M11-B06` each already flag it, never invented here).
3. **The mixed-session, cross-edition leg (AC3/AC4)** — the *evaluator* (a pure, self-tested function reading a Java bot's and a Bedrock bot's own independently-observed event logs and asserting each edition's client observed the other's action within a bounded tick window) and the *world-state comparator* (extending `M5-B10`'s own `hash_block_state_column`/`hash_biome_column` with a third, block-entity-NBT layer, per TEST-D10's own general formula) are both real and self-tested (Context §G/§H); the *live round trip* driving two real bots against one real server inherits the identical composition-root gap named in item 2, and is likewise honestly gated.
4. **The tier-conformance suite (AC5)** — real, Tier-1, **not gated** at all: `M11-B05`'s own already-real `TIER_TABLE`/`translate_*` functions are cited and cross-checked by this blueprint's own independent verifier (`verify_tier_conformance`, Context §I), proving CROSS-D15's Tier-1 set and CROSS-D16/D17's Tier-2/3 set are each *asserted present-and-bounded*, exactly the roadmap's own AC5 wording, today.
5. **The inertness leg (AC1)** — extends `M11-B06`'s own already-real `crossplay_disabled_binds_zero_bedrock_sockets_and_loads_zero_mapping_tables` suite with a runtime `tracing`-target scan (mirroring `M7-B09`'s own AC4a technique, restated for Bedrock) and a real `criterion` zero-cost benchmark (CROSS-D26's own second obligation), **plus** the full Java-only regression rerun — this blueprint's own justified extension of `M7-B09`'s own AC4b precedent, re-invoking `M7`'s own completion report unmodified against the identical crossplay-enabled-but-disabled binary (Context §E/§L).
6. **The cluster-mode leg (AC6)** — a `deploy/cluster/docker-compose.m11-acceptance.yml` overlay extending `M7-B08`'s base topology, wired, correct-by-construction, and failing closed with the exact, actionable `ClusterIntegrationPending` message this blueprint restates from `M7-B09`, **doubly** gated: `M7-B08`'s own still-open cluster-integration gap, *and* item 2's own still-missing translator wiring (Context §J).
7. **The manual leg (AC7)** — `docs/MANUAL-VERIFICATION-M11-B07.md`, CROSS-D25's one irreducible manual-verification step, documented in full even though, honestly, it cannot currently be executed to a real pass either (Context §K).
8. **The `xtask m11-report` completion report** (Context §M), continuing the established `M<n>ReportResult` shape, plus the **Roadmap Completion rollup** — the first machine-readable artifact in this corpus stating, explicitly, that the roadmap's `M0`–`M11` sequence — every milestone this project's planning corpus defines — has reached its own final node.
9. **Five mandatory harness self-tests** (Context §I/§E/§G, Acceptance tests), each proving a named failure mode this blueprint's own gates exist to catch is actually caught: a translation regression fails the tier-conformance suite; a leaked Bedrock thread under `crossplay = false` fails the inertness leg; a Bedrock-invisible-to-Java fixture fails the mixed-session evaluator; a divergent world-state fixture fails the cross-edition comparator; a stale Roadmap Completion input fails closed rather than assuming `pass`.

**The one genuine, honestly-disclosed, corpus-wide gap this blueprint depends on and does not close** (Context §A, restated in full there): **no blueprint through `M11-B06` provides a concrete `BedrockTranslator` implementation** — the trait `M11-B06` defines and deliberately leaves at `UnavailableBedrockTranslator` — that wires `M11-B05`'s own already-real, already-tested pure translation functions into `rusty-clanker-server`'s real ECS/session-intake path. This is not a gap this blueprint introduces; it is the gap `M11-B05`'s own Interfaces section names ("a future composition-root/connection-driver blueprint... a future Stage-11-integration blueprint") and `M11-B06`'s own Constraints (f) names ("do not build `rc-bedrock-translator` or any Play-state translation logic of any kind... `UnavailableBedrockTranslator` is the correct, honest current behavior, not a shortcut"), restated here as the single, precise, named contract every one of this blueprint's own gated cases waits on. A **second**, independent, already-inherited gap (Context §A item 2, restated from `M7-B09` §A, never re-derived) affects only the cluster-mode leg: `M7-B08`'s own still-open `main.rs` role-wiring gap.

Done when:

- [ ] `cargo build -p rc-paritybot -p xtask --all-features` succeeds with zero warnings, on both `ubuntu-24.04` and `windows-2025`.
- [ ] `cargo bench -p rusty-clanker-server --no-run --features "monolithic crossplay"` and `cargo bench -p rusty-clanker-server --no-run --no-default-features --features monolithic` both succeed with zero warnings (the new `crossplay_zero_cost` benchmark compiles under both feature sets).
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-paritybot -p xtask -p rusty-clanker-server -p rc-proxy`, using **only** synthetic in-memory data or a real, `--offline`, single-process `rusty-clanker-server` subprocess this blueprint itself spawns — no docker, no compose, no real pinned-version Bedrock client, required to go green.
- [ ] Every pre-existing `M11-B01`–`M11-B06` test still passes, byte-for-byte unmodified.
- [ ] The five mandatory harness self-tests (`translation_regression_fails_the_tier_conformance_suite`, `leaked_bedrock_thread_fails_inertness`, `bedrock_invisible_to_java_fails_the_mixed_session_evaluator`, `divergent_world_state_fails_the_cross_edition_comparator`, `missing_milestone_report_fails_the_roadmap_gate_closed`) all pass.
- [ ] `bedrock_bot_completes_full_lifecycle_then_receives_honest_unavailable_disconnect` passes: a real `BedrockBot` against a real, `--offline`, `crossplay`-enabled `rusty-clanker-server` subprocess completes `RequestNetworkSettings`→`NetworkSettings`→`Login`→`ServerToClientHandshake`/`ClientToServerHandshake`→`ResourcePacksInfo`/`Stack`/`ClientResponse` (`M11-B06` §E's algorithm, client side) and then receives, decrypted and decoded, exactly `UnavailableBedrockTranslator`'s own `DisconnectPacket`, with its exact configured message text, never a hang, never a `PlayStatus(LoginSuccess)`/`StartGame`.
- [ ] `verify_tier_conformance` (Context §I) passes against `rc_bedrock_translator::TIER_TABLE`'s own real, current content, cross-checking every row's claimed `Tier` against the actual observed output of its own `implemented_by`-named function on a small, hand-authored fixture — zero drift.
- [ ] `cargo run -p xtask -- m11-report --out-dir <dir>` (no `--compose`, no `--manual-evidence`) runs every Tier-1-provable leg for real and writes `target/verify/m11-acceptance.json`; AC2's `spawn_into_terrain` case, AC3's `live_round_trip` case, AC4's `live_world_state_comparison` case, AC6's cluster-mode case, and AC7 all report `fail` with the exact, actionable, gap-citing message — this is this blueprint's own correct, expected Done state until the named future blueprint(s) land, not a defect.
- [ ] `cargo run -p xtask -- m11-report --out-dir <dir> --regression-server-bin <same-binary>` folds in the M7 regression re-invocation (Context §L) via a real `xtask m7-report --criterion 4` subprocess call; its own `Ac4Report` (M7's own already-real leg) is embedded verbatim.
- [ ] `cargo run -p xtask -- m11-report --out-dir <dir> --roadmap` reads `target/verify/m1-acceptance.json`…`m10-acceptance.json` (whichever are present on disk) plus this run's own `m11-acceptance.json`, and produces a `RoadmapCompletionGate` whose `roadmap_complete` field is `false` today (since M11's own `overall` is `Fail` while any gated case remains open) — correct and expected.
- [ ] `cargo run -p xtask -- path-guard` exits 0 against this blueprint's own changeset (labeled per Constraints) — `crates/testing/paritybot/**`'s already-committed `PROTECTED_PATHS` row (`M1-B06`) already covers every new path this blueprint adds under that crate, proven by `path_guard_already_covers_this_blueprints_new_paths`.
- [ ] `.github/workflows/ci.yml`'s `on.workflow_dispatch.inputs.job` choice gains `m11-acceptance` as a fifth option (joining `reference-host-gate`/`release`/`compose-topology-gate`/`m7-acceptance` already there), and the new `m11-acceptance-gate` job is gated on `inputs.job == 'm11-acceptance'` — a YAML-parse check, not a runtime CI assertion.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-paritybot` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`, `path-guard`) green on both `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D50). The new `m11-acceptance-gate` job is `workflow_dispatch`-only, not part of the required Tier-1 status-check set; its own first meaningfully-green run for the cluster-mode leg is a **later** milestone-acceptance signal, gated on both named gaps landing — never a condition of this blueprint's own Done state.

## Context (self-contained)

### §A — Scope boundary: what this blueprint measures, and the two gaps it inherits (never introduces)

This blueprint builds **zero** new Bedrock-side production behavior. Every mechanism M11's seven acceptance criteria measure already exists, fully specified, in `M11-B01`–`M11-B06` — this blueprint's own job is precisely "wire the measurement," the identical role `M3-B08`/`M6-B06`/`M7-B09`/`M9-B07`/`M10-B06` each already played for their own milestone. Two genuine gaps in the prerequisite chain are inherited, restated once here rather than re-explained at each criterion they affect:

1. **No blueprint through `M11-B06` supplies a concrete `BedrockTranslator` implementation.** `M11-B06` Context §F defines the trait precisely — *"the seam a future `rc-bedrock-translator` blueprint... fills in with real Bedrock↔Java Play-state translation... A future concrete implementation is expected to translate `handoff.profile`... into the SAME `PlayerProfile` shape `crate::play::connection::enter_play` already consumes, and route it through the SAME `PlayerSessionSink`/join-time-resolution path `M6-B07` §H already established... never a parallel ECS ingress path"* — and ships exactly one implementation, `UnavailableBedrockTranslator`, whose own `accept` method sends a `DisconnectPacket` and closes, **by design**, restated by that blueprint as *"the correct, honest current behavior, not a shortcut around building the real thing."* `M11-B05`'s own Interfaces section independently names the identical gap from the other side, split into **two** distinct future pieces: *"Provides to a future composition-root/connection-driver blueprint: the complete session state machine... reading `RaknetSession::recv()`... calling `rc_bedrock_protocol`... converting this crate's own `Translated*`/`JavaSection`/`JavaMetadataValue` outputs to and from whatever real `rc-scheduler`/`rc-mechanics`/`rc-chunk-storage`-resident types the engine exposes"* and *"Provides to a future Stage-11-integration blueprint: `translate_section`/`translate_entity_*`/... as the pure encode step that blueprint's own dirty-generation-keyed shared-encode cache... wraps."* This blueprint's own binding resolution, restated for the fifth time in this corpus's own M11 lineage: **it is not this blueprint's job to close it.** A harness blueprint's role is to pin the missing contract precisely, prove everything provable without it, and fail closed — done here, exhaustively, for every one of AC2–AC4, AC6, and AC7 (§F–§H, §J, §K).
2. **`M7-B08`'s own still-open `main.rs` role-wiring gap**, restated verbatim from `M7-B09` §A rather than re-derived: no concrete, real-network `openraft::RaftNetworkFactory`/`JoinClient` exists, and `rc-proxy`'s own runtime wiring into `rusty-clanker-server::main.rs`'s `ServerRole::ClusterProxy`/`ClusterNode` arms does not exist either — both arms exit `EXIT_CLUSTER_INTEGRATION_PENDING` (code `3`). This affects only this blueprint's own cluster-mode leg (§J), which — per `M11-B06` §H's own already-established finding — inherits it doubled with gap 1 above, since even `M11-B06`'s own real, library-level proxy-side Bedrock relay (`proxy_relays_bedrock_login_to_fake_node`) proves only login-and-opaque-byte-relay, never a real translated Play-state on the node side.

Every one of this blueprint's own **live-round-trip-through-a-real-translator** cases therefore fails closed, honestly and identically, citing gap 1 (or, for the cluster leg, both gaps) — never a placeholder pass, never a silent skip. Every one of this blueprint's own **hermetically provable** cases — the bot driver's own send/decode correctness, the full RakNet/login/handshake/resource-pack lifecycle, the tier-conformance suite, the world-state comparator, the inertness leg, the Java-only regression rerun, the Roadmap Completion rollup's own read-and-aggregate logic — is real, Tier-1, and green today, built entirely against real M11-B01–M11-B06 code.

**A third, narrower, corpus-wide (not M11-specific) gap this blueprint names honestly rather than silently assumes closed:** `09-testing-quality.md`'s own TEST-D7/TEST-D8/TEST-D11 — the full differential-testing harness comparing a live Rusty Clanker process against a live, legally-downloaded vanilla `server.jar` subprocess, and the general-purpose RON scenario-corpus parser TEST-D11 describes — has **not** been built as concrete code by any blueprint through `M10` (verified by exhaustive search of every committed blueprint as of this derivation, 2026-08-24; only `rc-paritybot`'s own `loadtest` module, a *bot-swarm load-generation* harness, and its own `idle_stability` connection-liveness check, both unrelated to differential vanilla comparison, exist today). This is fortunate rather than blocking for CROSS-D24's own specific text — restated precisely in §G below — which compares two Rusty-Clanker-driven runs against **each other**, never against vanilla, so it needs no vanilla-jar oracle at all. This blueprint's own cross-edition scenario format (§G) is therefore its own necessary, minimal, honestly-scoped instantiation of TEST-D11's own decision, designed as a natural subset a future general differential-harness blueprint can absorb without a rewrite — flagged for reconciliation into `09`'s own next revision (Open Questions), never presented as though TEST-D11's full corpus format already exists.

### §B — M11's seven acceptance criteria, verbatim, and this blueprint's own precise reading of each

From `11-roadmap-milestones.md`'s M11 section (itself `15-crossplay.md`'s own CROSS-D22 text, incorporated verbatim), quoted in full:

1. *"`crossplay = false` (or absent): an automated test confirms no UDP socket is bound on the configured RakNet port; a `criterion` benchmark shows no measurable tick-time regression against a `crossplay`-feature-stripped build (CROSS-D26)."*
2. *"`crossplay = true`, monolithic mode: an unmodified, pinned-version Bedrock client completes the full connection lifecycle (RakNet handshake, JWT-chain login, encryption handshake, resource-pack negotiation) and spawns into the server's real, `M5`-generated terrain — not a placeholder or superflat world."*
3. *"A scripted `both-simultaneously` cross-edition scenario (CROSS-D24) confirms a block placed by the Java client is observed by the Bedrock client, and vice versa, within a bounded tick window, with resulting world state identical regardless of which edition performed the action."*
4. *"The full CROSS-D15 Tier-1 behavior set passes the cross-edition consistency suite (CROSS-D24) with world-state hashes identical to the Java-only baseline (`09`'s TEST-D10)."*
5. *"The full CROSS-D16/D17 Tier-2/3 set is asserted present-and-bounded via a dedicated fixed test matrix — any newly discovered Bedrock-side limitation is added to the tier table (CROSS-D18) before the affected feature is considered acceptance-complete."*
6. *"`crossplay = true`, cluster mode: a Bedrock client's session survives a cross-node region-boundary handoff within `M7`'s already-proven CLUSTER-D22 ≤2-tick, zero-disconnect budget, verified by a Bedrock-bot variant of `09`'s TEST-D21 handoff suite."*
7. *"A real, unmodified pinned-version Bedrock game client manually joins, renders, and plays a continuous session against a full-featured (`M0`–`M7`) Rusty Clanker build (CROSS-D25's one manual-verification carve-out)."*

**This blueprint's own binding AC-to-`m11-report`-section mapping**, restated once and reused throughout: AC1→§E, AC2→§F, AC3→§G, AC4→§H, AC5→§I, AC6→§J, AC7→§K. AC1 and AC5 are fully real and provable today; AC2's lifecycle half is real, its "spawns into terrain" half is gated on §A gap 1; AC3 and AC4 have real, self-tested evaluators/comparators but gated live round trips (§A gap 1); AC6 is gated on both §A gaps; AC7 is gated on §A gap 1 (a real client cannot reach Play state today either) — every gated case's own `fail` status names its exact §A citation, never a generic "not implemented" (mirroring `M9-B07`/`M10-B06`'s own identical discipline).

### §C — CROSS-D23, restated in full and implemented

CROSS-D23, quoted verbatim: *"Test methodology: a Bedrock bot driver built on the project's own `rc-bedrock-protocol`/`rc-bedrock-raknet` crates, added as a dev-dependency of `rc-paritybot`... not a new third-party dependency. Unlike TEST-D8's Java-side `azalea`... no comparably mature, independently-maintained **Rust** Bedrock bot library exists as of this research (the mature options — `PrismarineJS/bedrock-protocol` in Node.js, `Sandertv/gophertunnel` in Go — would require a cross-language subprocess harness rather than a pure-Rust dev-dependency); the crossplay bot driver therefore necessarily dogfoods the same crates it is meant to validate, a known, explicitly acknowledged reduction in test independence relative to TEST-D8's Java-side signal."* This blueprint restates that reality honestly, exactly as CROSS-D23's own rationale text demands (*"Naming this trade-off explicitly... is what keeps the project's... documentation discipline honest here — the mitigation (CROSS-D25's mandatory real-client manual pass) exists specifically because this decision's automated signal is weaker than Java's own"*): every assertion `BedrockBot`-driven test in this blueprint proves that Rusty Clanker's **own** hand-written `rc-bedrock-raknet`/`rc-bedrock-protocol`/`rc-bedrock-auth` codec round-trips **against itself**, correctly — a genuine, valuable, zero-cost signal for catching an internal regression the moment it lands, but **not** independent confirmation that a genuine Mojang-built Bedrock client would accept the same bytes. That confirmation is exactly, and only, CROSS-D25's manual pass (§K) — restated as this blueprint's own binding acknowledgment, never silently presented as equivalent-strength evidence to TEST-D8's own independently-maintained `azalea` signal.

**Crate placement, restated precisely** (CROSS-D23's own "dev-dependency of `rc-paritybot`" text, read against this crate's own already-established practice, `M1-B06`/`M6-B01`): `azalea` — TEST-D8's own Java-side bot library — is consumed by `rc-paritybot`'s **real, non-test-`cfg`'d** `src/loadtest/`/`src/idle_stability.rs` modules, not merely by its `tests/` tree; "dev-dependency," in TEST-D8/CROSS-D23's own usage, therefore means *"a dependency scoped entirely to the testing crate, never reachable from any shipped runtime crate,"* not literally Cargo's `[dev-dependencies]` manifest section (which cannot be imported by a crate's own `src/` at all). This blueprint's own `rc-bedrock-raknet`/`rc-bedrock-protocol`/`rc-bedrock-auth` additions to `rc-paritybot`'s `Cargo.toml` follow the identical, already-established convention — ordinary `[dependencies]` entries, restricted from ever reaching `rusty-clanker-server`'s own shipped binary by the simple, structural fact that `rc-paritybot` is never a dependency of it (the same "restricted by dependency-graph shape, not by convention" property CROSS-D5/WS-D3 already give the production Bedrock crates, here applying in the opposite direction: nothing *shipped* ever depends on `rc-paritybot`). No new external dependency, no `[workspace.dependencies]` edit, no `cargo-deny` `bans` rule change is needed — every crate `BedrockBot` uses is already workspace-pinned by `M11-B01`/`M11-B02`/`M11-B03`.

### §D — `BedrockBot`: the client-side driver, restated once, reusing `rc-bedrock-auth`'s own math directly

**A load-bearing design choice, stated once and applied throughout:** `M11-B06`'s own `tests/support/bedrock_fake_client.rs` (`FakeBedrockClient`) already drives the identical client-side algorithm this blueprint needs — but it lives inside `crates/server/tests/`, a private test binary `rc-paritybot` has no Cargo path to (mirroring the exact "restated, not shared, because of dependency-direction constraints" reasoning `M11-B01` §A item 1/`M11-B02` §A/`M11-B03` §A/`M11-B05` §A/`M11-B06` §A item 1 each already establish for their own analogous situations). `BedrockBot` is therefore a **fresh, independent, real** implementation of `M11-B06` §E's algorithm from the client's own side — restated a sixth time in this corpus's own M11 lineage, never imported. Unlike `FakeBedrockClient`, `BedrockBot` is a first-class, reusable **library type**, promoted out of any one blueprint's own test-support module, exactly the "restated once, as real reusable code, not re-duplicated per test file" discipline this blueprint's own role as the *final* M11 harness earns it the right to do.

**§D.1 — What is genuinely new (chain signing) vs. what is real, honest reuse (ECDH/AEAD).** `rc_bedrock_auth::chain::validate_chain` only *verifies* a chain — CROSS-D5 rule 5 scopes that crate as a server-side verification toolkit, and it exposes no client-side signing API at all (restated from `M11-B03`'s own Context, "why this crate never depends on `rc-bedrock-protocol`," which never claims a signing role either). `BedrockBot`'s own `chain_signing.rs` (new, this blueprint's own necessary addition) is a small, hand-rolled JWT-compact-serialization builder — structurally **identical** to `M11-B03`'s own already-committed test-only `make_claim` helper (`crates/bedrock-auth/tests/support.rs`, restated by reference rather than reproduced, since `tests/support/` is likewise not a library `rc-paritybot` can import), elevated here to real, non-test-gated code: given a fresh `p384::SecretKey` and a `serde_json::Value` payload, produces `base64url(header) + "." + base64url(payload) + "." + base64url(raw_96_byte_es384_signature)`, header always `{"alg":"ES384","x5u": base64(spki_der_of_the_signing_key)}` (`M11-B03`'s own restated field shape, §"`chain`" section). **This blueprint's own bot always constructs a single-claim, self-signed chain** (`chain.len() == 1`, payload `{"displayName": <bot username>}`), the identical unauthenticated shape `M11-B06`'s own `FakeBedrockClient` already uses, and therefore **only ever exercises `auth_mode = "offline"` servers** — exactly the same restriction TEST-D8's own Java bot already carries for its own `--offline` differential runs (`M9-B07` §Context 8's own "this blueprint's own `--offline` `RealServer` sessions never hold a real Microsoft account" framing, restated here for the Bedrock side) — restated as this blueprint's own explicit, binding scope line, never silently expanded: a real, online, Mojang-root-key-anchored chain needs a genuine, signed-in Xbox Live account no automated bot can hold, exactly CROSS-D25's own reason for existing.

The **encryption handshake's own math**, by contrast, needs **no** new code at all: `rc_bedrock_auth::handshake::{ServerEcdhKeyPair, SharedSecret, generate_salt}`'s own ECDH/session-key-derivation implementation is symmetric in its actual computation (a fresh keypair, `diffie_hellman(peer_public_key_der)`, `derive_session_key(salt)`) regardless of which side of the connection constructed it — the type's name reflects *M11-B03's own original consumer* (the server), not a mathematical asymmetry in what it computes. `BedrockBot` therefore calls `rc_bedrock_auth::handshake::ServerEcdhKeyPair::generate()` for its **own** ephemeral keypair, `diffie_hellman(&server_public_key_der_extracted_from_the_ServerToClientHandshakePacket's_own_x5u_header)` for the shared secret, and `derive_session_key(&salt_extracted_from_that_same_token's_payload)` for the session key — then constructs `BedrockAeadEncryptor`/`BedrockAeadDecryptor` (`M11-B03`, reused directly, unmodified) from that key, exactly as the server does from the *other* side of the identical math. This is genuine, correctness-preserving reuse of one already-tested implementation — never a second, independently-authored (and therefore independently-buggy-prone) reimplementation of AES-256-GCM/ECDH, the one place in this blueprint where "restate, don't share" is deliberately **not** applied, because nothing about the math itself needs restating.

```rust
// crates/testing/paritybot/src/bedrock_bot/mod.rs (new)

pub struct BotIdentity {
    pub display_name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum BedrockBotError {
    #[error("RakNet handshake failed: {0}")]
    RaknetHandshake(String),
    #[error("login sequence error at step {step}: {message}")]
    Login { step: &'static str, message: String },
    #[error("unexpected packet: expected {expected}, got id {actual_id}")]
    UnexpectedPacket { expected: &'static str, actual_id: u16 },
    #[error("connection closed before the expected packet arrived")]
    Closed,
    #[error("packet decode error: {0}")]
    Decode(String),
}

/// One end-to-end Bedrock client session driver (CROSS-D23), built entirely on
/// `rc-bedrock-raknet`/`rc-bedrock-protocol`/`rc-bedrock-auth`'s own real, already-shipped
/// public APIs — never a stub, never a second wire-format implementation.
pub struct BedrockBot {
    session: rc_bedrock_raknet::RaknetSession,
    compression: rc_bedrock_protocol::CompressionAlgorithm,
    encryptor: Option<rc_bedrock_auth::handshake::BedrockAeadEncryptor>,
    decryptor: Option<rc_bedrock_auth::handshake::BedrockAeadDecryptor>,
    identity_key: p384::SecretKey,
}

pub struct LoginObservation {
    pub received_disconnect: Option<rc_bedrock_protocol::login::DisconnectPacket>,
    pub received_play_status_success: bool,
    pub received_start_game: Option<rc_bedrock_protocol::startgame::StartGamePacket>,
}

impl BedrockBot {
    /// Completes M11-B01's own offline+online RakNet handshake against `addr` (`RaknetSession`
    /// reaches `Connected`, M11-B01 §I) — restates no wire fact of its own, calls straight
    /// through to that crate's own client-side handshake helpers.
    pub async fn connect(addr: std::net::SocketAddr, runtime: tokio::runtime::Handle) -> Result<Self, BedrockBotError>;

    /// Drives `M11-B06` §E's algorithm, client side, steps 1-8: sends `RequestNetworkSettings`,
    /// receives `NetworkSettings`, sends a self-signed single-claim `LoginPacket` (Context §D.1)
    /// plus a self-signed client-data token, receives `ServerToClientHandshake`, installs
    /// encryption (Context §D.1's own reuse of `rc_bedrock_auth::handshake`), sends
    /// `ClientToServerHandshake`, negotiates resource packs (always the zero-packs default path,
    /// `ResourcePackClientResponse{response: ResourcePackStackFinished}` immediately), then reads
    /// **one** further packet and classifies it into `LoginObservation` — `DisconnectPacket`
    /// (today's honest outcome, §F), `PlayStatus(LoginSuccess)`+`StartGame` (the outcome once the
    /// composition-root/translator gap, §A, closes), or `Err(Closed)` if the connection drops
    /// with no reply at all (a real bug, never conflated with an honest, explicit disconnect).
    pub async fn complete_login(&mut self, identity: BotIdentity) -> Result<LoginObservation, BedrockBotError>;

    /// Sends one `PlayerAuthInputPacket` (M11-B02 §Q) built from the given position/rotation and
    /// input-intent flags — a well-formed wire send; whether any server-side consequence follows
    /// depends entirely on whether Play state was ever reached (§A gap 1).
    pub async fn send_movement(&mut self, position: (f64, f64, f64), rotation: (f32, f32), flags: rc_bedrock_protocol::movement::PlayerAuthInputFlags) -> Result<(), BedrockBotError>;

    /// Sends one `PlayerAuthInputPacket` carrying exactly one `PlayerBlockAction` (M11-B02 §Q) —
    /// the Bedrock-side analog of a Java client's `Player Action`/`Use Item On`.
    pub async fn send_block_action(&mut self, action: rc_bedrock_protocol::movement::PlayerBlockAction) -> Result<(), BedrockBotError>;

    /// Sends one `TextPacket{text_type: Chat, ..}` (M11-B02 §S) with the given message body.
    pub async fn send_chat(&mut self, message: &str) -> Result<(), BedrockBotError>;

    /// Decrypts (if encryption is active), decompresses, and decodes exactly one further
    /// sub-packet, blocking up to `timeout`. `Ok(None)` on a clean session close.
    pub async fn recv_raw(&mut self, timeout: std::time::Duration) -> Result<Option<(rc_bedrock_protocol::packet::PacketHeader, bytes::Bytes)>, BedrockBotError>;

    /// Graceful `Disconnection Notification` then RakNet teardown (M11-B01 §I).
    pub async fn disconnect(mut self);
}
```

```rust
// crates/testing/paritybot/src/bedrock_bot/chain_signing.rs (new)

/// Restates `M11-B03`'s own already-committed test-only `make_claim` helper (`crates/bedrock-auth/
/// tests/support.rs`) as real, reusable code — no Cargo path exists from `rc-paritybot` to that
/// private test binary, so this is a fresh, independent authoring of the identical, small,
/// hand-rolled JWT-compact-serialization construction, never a reproduction under a different
/// license or a copy-paste (ASSET-D18/D19, Constraints).
pub fn build_self_signed_claim(signing_key: &p384::SecretKey, payload: serde_json::Value) -> String;

/// This blueprint's own single-claim, unauthenticated identity chain (Context §D.1) — the
/// complete `LoginPacket.chain` value `BedrockBot::complete_login` sends.
pub fn build_offline_chain(signing_key: &p384::SecretKey, display_name: &str) -> Vec<String>;

/// A minimal, self-signed client-data token — `rc_bedrock_auth::client_data`'s own consumed
/// field list, this blueprint's own baseline populates only what the corresponding acceptance
/// tests actually inspect (`DeviceOS`, `SkinId`), every other client-data field a real client
/// sends left absent, an explicit, bounded simplification (never claimed exhaustive).
pub fn build_offline_client_data_token(signing_key: &p384::SecretKey) -> String;
```

### §E — AC1: the inertness leg, extended (real, Tier-1)

**§E.1 — Runtime `tracing`-target scan, extending `M11-B06`'s own compile/config-level proof with a process-level one.** `M11-B06` Context §D already proves, statically and by construction, that no `RaknetListener::bind` call site is ever reached with `crossplay.enabled == false` (Context §D item 1) and that `MappingTables::load()` has zero call sites in the workspace at all (item 2). This blueprint adds the direct, **runtime** complement — mirroring `M7-B09` §H.2's own identical technique for cluster mode, restated for Bedrock's own marker set:

```rust
// xtask/src/m11_report.rs (new)

/// A simple, deliberately conservative substring scan over a real, running server's own
/// captured `tracing` output (mirroring `M7-B09`'s `scan_for_cluster_targets` exactly, restated
/// for a different marker set) — `Some(line)` names the first offending line, `None` if clean.
pub fn scan_for_bedrock_targets(log_lines: &[String]) -> Option<String>;

const BEDROCK_TARGET_MARKERS: &[&str] = &["bedrock", "rc_bedrock", "raknet"];

/// A `rusty-clanker-server` process started with `crossplay.enabled: false` (or `[crossplay]`
/// absent) is probed on two axes, mirroring `M7-B09` §H.2's own two-part AC4a check exactly:
/// (1) `scan_for_bedrock_targets` over its captured stdout/stderr finds nothing; (2) a TCP/UDP
/// connect attempt against the configured `crossplay.bind` port is refused (`ConnectionRefused`),
/// never a live listener. Both are pure, synthetic-data-testable functions exercised directly by
/// this blueprint's own mandatory self-test (Acceptance tests) without needing a real process.
pub fn evaluate_inertness(log_lines: &[String], bedrock_bind_probe: PortProbeResult) -> tier_result::TierResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortProbeResult { Refused, UnexpectedlyOpen }
```

**§E.2 — The `criterion` zero-cost benchmark (CROSS-D26's own second obligation).** `crates/server/benches/crossplay_zero_cost.rs` (new — `criterion` per TEST-D29, already workspace-pinned, zero new dependency) drives `ServerComposition`'s own already-real, library-constructible synchronous test-mode tick loop (`M6-B07`/`M6-B01`'s own established "construct as a library type, drive N ticks directly, never through `main.rs`" pattern, restated for a feature-set comparison rather than a config comparison) over a small, fixed, empty-world synthetic scenario, for `ZERO_COST_BENCH_TICKS = 200` ticks — **once per Cargo compilation**, never per runtime config: this benchmark file itself references no Bedrock type at all (it drives the ordinary tick loop identically regardless of whether `crossplay` is compiled in), so the same, byte-identical file is compiled and run twice, under two **separately invoked** `cargo bench` commands with two different feature sets:

```
cargo bench -p rusty-clanker-server --features "monolithic crossplay" --bench crossplay_zero_cost   # crossplay code present, `[crossplay]` absent at runtime (CROSS-D4's default-off, exercised here)
cargo bench -p rusty-clanker-server --no-default-features --features monolithic --bench crossplay_zero_cost   # crossplay code entirely absent (WS-D5(e)'s strip case)
```

Each run writes `criterion`'s own standard machine-readable estimate (`target/criterion/crossplay_zero_cost/base/estimates.json`) to a distinct output directory (`criterion`'s own `--output-format`/baseline-naming flags, no new mechanism). `xtask m11-report`'s own `run_zero_cost_benchmark` (Deliverables) shells out to both commands in turn, parses both `estimates.json` files, and computes the percentage delta between the two runs' own mean tick time — reusing TEST-D29's own already-established **≥5% regression fails** threshold verbatim (never a new, invented tolerance), the same "reuse an already-reviewed number rather than invent a second one" discipline `CROSS-D16`'s own rationale text already applies to reusing GeyserMC's published flood-protection figures.

```rust
#[derive(serde::Deserialize)]
struct CriterionEstimate { mean: CriterionPointEstimate }
#[derive(serde::Deserialize)]
struct CriterionPointEstimate { point_estimate: f64 } // nanoseconds, criterion's own convention

/// Pure: `Err` names which side regressed and by how much if the delta exceeds TEST-D29's own
/// ≥5% threshold; `Ok(delta_percent)` otherwise (a negative `delta_percent` — crossplay-compiled-
/// in-but-disabled measuring *faster* than the stripped build — is accepted without complaint,
/// exactly as ordinary run-to-run noise permits either sign within tolerance).
pub fn compare_zero_cost_estimates(crossplay_enabled_ns: f64, crossplay_stripped_ns: f64) -> Result<f64, ZeroCostRegression>;

#[derive(Debug, PartialEq)]
pub struct ZeroCostRegression { pub delta_percent: f64 }
```

**§E.3 — The Java-only regression rerun, restated and extended from `M7-B09`'s own criterion-4 pattern.** Restated in full, §L below.

### §F — AC2: the join lifecycle, live; "spawns into terrain," honestly gated

`bedrock_bot_completes_full_lifecycle_then_receives_honest_unavailable_disconnect` (Acceptance tests, Done-when) is this blueprint's own real, live proof of AC2's entire first clause: a real `BedrockBot` against a real, `--offline`, `crossplay`-enabled `rusty-clanker-server` subprocess (`M3-B08`'s own established `ManagedServer` wrapper, reused unmodified) completes every step of `M11-B06` §E's algorithm for real — RakNet handshake through `ClientToServerHandshake`, real ECDH/AES-GCM installation (via §D's own reused `rc_bedrock_auth::handshake` types on both ends of the same connection, independently derived, converging on the identical session key — the strongest possible internal-consistency proof this blueprint can offer, per §C's own honest limit), real resource-pack negotiation over the zero-packs default (`M11-B06` §I). `m11_report::run` (Deliverables) folds this real leg into AC2's own `Ac2Report.automated` (`TierResult`, `tier: "m11-ac2"`), case `lifecycle_completes`, `Status::Pass`.

`Ac2Report`'s own second case, `spawn_into_terrain`, is this blueprint's own honestly-gated one: it reports `Status::Fail`, `detail: Some(AC2_SPAWN_GATE_MESSAGE)`, unconditionally, until a future composition-root/ECS-adapter/Stage-11-integration blueprint lands and this blueprint's own implementer is asked to wire a real evidence path (mirroring `M9-B07`/`M10-B06`'s own established `--manual-evidence`-flag pattern, restated identically here — Deliverables' own `M11ReportArgs.manual_evidence` field).

```rust
pub const AC2_SPAWN_GATE_MESSAGE: &str = "AC2's own 'spawns into the server's real, M5-generated \
    terrain' clause needs a concrete BedrockTranslator implementation (M11-B06 Context §F's own \
    UnavailableBedrockTranslator is the correct, honest current behavior) wired into a \
    composition-root/ECS-adapter blueprint and a Stage-11-integration blueprint (M11-B05's own \
    Interfaces section names both), neither of which is merged as of M11-B07's own drafting. This \
    blueprint's own bedrock_bot_completes_full_lifecycle_then_receives_honest_unavailable_disconnect \
    test proves every OTHER clause of AC2 real and green today.";
```

### §G — AC3: the mixed-session cross-edition leg — a real, self-tested evaluator; a gated live round trip

**§G.1 — This blueprint's own minimal `CrossEditionScenario` format**, CROSS-D24's own necessary, honestly-scoped instantiation (§A's third gap, restated): a small, hand-authored RON fixture naming one fixed world position, which edition performs which scripted action at which tick, and the bounded observation window both editions' own bots are checked against.

```rust
// crates/testing/paritybot/src/bedrock_bot/mod.rs (continued)

#[derive(serde::Deserialize, Debug, Clone)]
pub struct CrossEditionScenario {
    pub name: String,
    /// `Java` places a block at `position`, `Bedrock` observes it (or vice versa) — the roadmap's
    /// own "both-simultaneously" text is realized as two runs of this same scenario with
    /// `actor`/`observer` swapped, never a single packet-level simultaneity claim this blueprint
    /// cannot actually measure without a live translator anyway.
    pub actor: Edition,
    pub position: (i32, i32, i32),
    pub block_name: String,
    /// Ticks after the actor's own action within which the observer must see it (CROSS-D24's own
    /// "within a bounded tick window" text, made concrete) — seed default, calibration-pending
    /// like every other numeric threshold in this corpus.
    pub observation_window_ticks: u32,
}
#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edition { Java, Bedrock }

pub const CROSS_EDITION_OBSERVATION_WINDOW_TICKS: u32 = 40; // 2s @ 20 TPS, seed default
```

**§G.2 — The pure evaluator**, the one this blueprint's own mandatory self-test (`bedrock_invisible_to_java_fails_the_mixed_session_evaluator`, Acceptance tests) drives directly against a synthetic, hand-built event log — **never** requiring a live server, a live translator, or even a compiled scenario file to exercise:

```rust
// xtask/src/m11_report.rs (continued)

/// One edition's own bot-observed event, independent of the other bot's own log (each bot
/// records only what it itself decoded off its own connection — never a merged, privileged view).
#[derive(Debug, Clone, PartialEq)]
pub enum CrossEditionEvent {
    /// The acting edition's own confirmation its action was sent (never that it was *applied* —
    /// that is exactly what the *other* edition's own observation, below, is the proof of).
    ActionSent { edition: Edition, tick: u32, position: (i32, i32, i32) },
    /// The observing edition's own decoded evidence the world changed at `position` — a Java
    /// bot's `Block Update`/`ChunkBatchFinished`-adjacent observation, or a Bedrock bot's own
    /// `UpdateBlockPacket` (M11-B02 §P) observation, each edition's own real wire packet, never
    /// a synthetic stand-in once the live leg is un-gated.
    WorldChangeObserved { edition: Edition, tick: u32, position: (i32, i32, i32) },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MixedSessionReport {
    pub actor_confirmed_send: bool,
    /// `true` iff the OTHER edition observed the change within `scenario.observation_window_ticks`
    /// of the actor's own send tick — the literal "observed... within a bounded tick window" claim.
    pub cross_edition_observation_confirmed: bool,
    pub passed: bool,
}

/// Pure: `passed = actor_confirmed_send && cross_edition_observation_confirmed`. The mandatory
/// self-test's own fixture — an event log containing `ActionSent{edition: Java, ..}` but zero
/// `WorldChangeObserved{edition: Bedrock, ..}` events at all (the literal "Bedrock-invisible-to-
/// Java" — or here, symmetrically, "Java-invisible-to-Bedrock" — fake) — must produce
/// `cross_edition_observation_confirmed == false`, `passed == false`.
pub fn evaluate_mixed_session(scenario: &CrossEditionScenario, actor_send_tick: u32, events: &[CrossEditionEvent]) -> MixedSessionReport;
```

**§G.3 — The live round trip.** `Ac3Report`'s own `live_round_trip` case reports `Status::Fail`, citing `AC2_SPAWN_GATE_MESSAGE` verbatim — a `both-simultaneously` scenario needs **both** a Java bot and a Bedrock bot to reach real Play state on the same real server, and the Bedrock half of that is exactly, and only, §A gap 1. `Ac3Report`'s own `evaluator_self_test` case (`Status::Pass`, sourced from the mandatory self-test itself) is what proves the evaluator is real and ready the moment the live leg un-gates.

### §H — AC4: the Tier-1 world-state comparator — real and self-tested; the live comparison, gated

**§H.1 — Extending `M5-B10`'s own hash primitives, restated once.** `M5-B10` Context §C already delivers `xtask::worldgen_corpus::{hash_block_state_column, hash_biome_column}` over `rc_chunk_storage::column::{BlockStateColumn, BiomeColumn}`, reused **unmodified** here — that blueprint's own doc comment explicitly anticipates exactly this extension ("a future revision... may add a third, diagnostic-only... hash without changing this blueprint's own gate definition"). This blueprint adds the third layer TEST-D10's own general formula names (*"block-state palette-indexed array, block-entity NBT, biome array"*) and `M5-B10` deliberately deferred (that blueprint's own gate is worldgen-specific and narrower than TEST-D10's general one, by its own explicit design):

```rust
// xtask/src/m11_report.rs (continued)

/// The third layer M5-B10's own gate deliberately excludes (worldgen has no meaningful
/// block-entity content at generation time) but TEST-D10's own general differential formula
/// requires — SHA-256 (reusing `xtask::fixture_manifest::compute_sha256_hex`, this project's own
/// single hand-rolled implementation, M0-B07, never a second hashing algorithm) over each block
/// entity's own `(local_pos, type_name, serialized NBT bytes)` triple, sorted by `local_pos`
/// first (a chunk's own block-entity list has no other canonical order) so two structurally
/// identical sets in different insertion order hash identically.
pub fn hash_block_entities(entities: &[BlockEntitySnapshot]) -> String;

#[derive(Debug, Clone, PartialEq)]
pub struct BlockEntitySnapshot { pub local_pos: (u8, u8, u8), pub type_name: String, pub nbt_bytes: Vec<u8> }

/// TEST-D10's own full three-layer content hash, restated exactly: reuses M5-B10's two functions
/// unmodified, adds this blueprint's own third — **lighting explicitly excluded**, per TEST-D10's
/// own text ("since lighting is derived state recomputed by a BFS fixed point, ARCH-D16, not
/// authoritative input/output state"), restated here rather than re-derived.
pub struct WorldStateHash { pub block_states: String, pub biomes: String, pub block_entities: String }
pub fn hash_world_state(blocks: &rc_chunk_storage::column::BlockStateColumn, biomes: &rc_chunk_storage::column::BiomeColumn, entities: &[BlockEntitySnapshot]) -> WorldStateHash;
```

**§H.2 — The mandatory self-test, `divergent_world_state_fails_the_cross_edition_comparator`**: two synthetic `BlockStateColumn` fixtures identical except for one cell's own value; `hash_world_state`'s own `block_states` field differs between the two — `assert_ne!`, proving the comparator is actually sensitive to a real, minimal divergence, never merely "returns a string."

**§H.3 — CROSS-D15's Tier-1 set, the static half (real, today), and the live half (gated).** `Ac4Report`'s own `tier1_set_present` case cites `M11-B05`'s own already-real `TIER_TABLE` rows tagged `Tier::Parity` (block place/break, redstone observation, basic inventory manipulation, plaintext chat, real worldgen terrain — `M11-B05` Context §N, restated by name, never re-derived) — `Status::Pass`, since every one of those rows already carries a real `implemented_by` function proven by `M11-B05`'s own `tier_table_conformance.rs`. `Ac4Report`'s own `live_world_state_comparison` case — actually driving a Java bot and a Bedrock bot through the same scenario against the same real server and calling `hash_world_state` on each edition's own resulting chunk — reports `Status::Fail`, `AC2_SPAWN_GATE_MESSAGE`, for the identical reason §G.3 names.

### §I — AC5: the Tier-2/3 degradation matrix (real, Tier-1, not gated)

**The one Play-state-adjacent acceptance criterion this blueprint proves live, end to end, today** — because it is, by CROSS-D16/D17/D18's own text, a claim about the *tier table's own internal consistency and completeness*, never about a live client round trip. `M11-B05`'s own `tier_table_conformance.rs` already asserts `TIER_TABLE`'s own structural well-formedness (every non-`Parity` row's `note` non-empty, every `Degraded`/`Unsupported` row names an implementing function or the one named exception). This blueprint's own contribution is an **independent** cross-check — mirroring TEST-D52's own "a second reviewing intelligence, not just a second mechanical pass" role, applied here as a second, independently-authored evaluator over the identical table rather than trusting `M11-B05`'s own self-report:

```rust
// xtask/src/m11_report.rs (continued)

/// This blueprint's own independent re-derivation of what `rc_bedrock_translator::TIER_TABLE`
/// claims — never imported from that crate's own test module, deliberately re-authored here so a
/// genuine drift between the table's own claim and its own function's real behavior cannot hide
/// behind one shared assertion (the identical "an independently-authored check, not the same code
/// path twice" discipline TEST-D9's own semantic packet comparator already applies at the
/// protocol layer, restated here at the tier-table layer).
pub fn verify_tier_conformance(table: &[rc_bedrock_translator::TierEntry], observed: &[ObservedTierBehavior]) -> tier_result::TierResult;

/// One row's own actual, harness-observed behavior — fed by this blueprint's own small fixture
/// drivers, each calling the named `implemented_by` function directly (`translate_offhand`,
/// `translate_outbound_chat`, `bridge_item_stack_request`'s own `RejectedUnsupportedAction` path,
/// etc. — `M11-B05`'s own already-real functions, called here, never reimplemented) and
/// classifying the result.
pub struct ObservedTierBehavior { pub feature: &'static str, pub observed_tier: rc_bedrock_translator::Tier }
```

**The mandatory self-test, `translation_regression_fails_the_tier_conformance_suite`**: a synthetic `TIER_TABLE`-shaped fixture claims one row `Tier::Parity`, but the matching `ObservedTierBehavior` fixture reports `Tier::Degraded` for that same feature (simulating a real future regression — a function that used to translate a feature fully but now silently drops part of it) — `verify_tier_conformance` must report that row `Status::Fail`, never silently accept the claim.

### §J — AC6: the cluster-mode leg, doubly gated (§A), the compose overlay specified in full

`deploy/cluster/docker-compose.m11-acceptance.yml` (new, non-code, Deliverables) is a **compose override file**, mirroring `M7-B09` §C's own identical technique exactly: layered over `M7-B08`'s own base `docker-compose.cluster-test.yml` (three `node-*` services, one `minio`), with `M7-B09`'s own `docker-compose.m7-acceptance.yml` (two `proxy-*` services) as a **third**, co-layered file — `docker compose -f docker-compose.cluster-test.yml -f docker-compose.m7-acceptance.yml -f docker-compose.m11-acceptance.yml` — adding exactly one additive field to each already-declared `proxy-*` service: a published UDP port for `[crossplay].bind` (`M11-B06`'s own `ProxyConfig.bedrock_bind` field, restated), plus a `[crossplay]` config block (`auth_mode = "offline"`, matching every other harness in this corpus's own established `--offline` convention) baked into each proxy's own mounted config file. This override changes nothing about either base file's own already-tested shape.

**Cluster-mode acceptance criterion, restated precisely, per CROSS-D22's roadmap text:** a `BedrockBot`, connected through one of the overlay's own two Bedrock-enabled proxy ports, is driven across a scripted region-border crossing exactly as TEST-D21's own Java-side handoff suite already specifies (*"an azalea bot crosses a region border under three synthetic load conditions... asserts: (i) the client's TCP/QUIC-fronted connection is never closed; (ii) client-observable rubber-banding stays within CLUSTER-D22's 2-tick budget... (iii) post-crossing entity/inventory/effect state matches a pre-crossing snapshot exactly; (iv) the... handoff sequence completes in the documented order"*), Bedrock-adapted: (i) becomes "the `RaknetSession` is never observed `Disconnected`"; (ii) is measured identically, off the bot's own received-packet timestamps; (iii) is measured via `hash_world_state` (§H) pre- and post-crossing; (iv) is unchanged, since `M11-B06` §H already confirms the six-step handoff protocol stays entirely protocol-agnostic (*"the handoff sequence was already, by construction, protocol-agnostic"*).

`m11_report::run_cluster_leg` (Deliverables) — mirroring `M7-B09`'s own `ClusterIntegrationPending` error exactly, restated for M11's own doubled gate:

```rust
#[derive(Debug, thiserror::Error)]
pub enum M11ReportError {
    #[error(
        "AC6's real leg needs a live, multi-node compose topology (M7-B08's still-open Context \
         §A items 1/3, see M7-B09 Context §A) AND a concrete BedrockTranslator implementation on \
         the node side (M11-B07 Context §A item 1, AC2_SPAWN_GATE_MESSAGE) — both are known, \
         tracked dependency gaps, not a bug in this harness. Run with --skip-cluster to exercise \
         every other leg, which is real and green today."
    )]
    ClusterAndTranslatorIntegrationPending,
    // ... ordinary I/O/build/spawn-failure variants, implementer's own freedom
}
```

CI-tier placement, restated per `M7-B09` §I's own established table: `deploy/cluster/docker-compose.m11-acceptance.yml`'s own structural validity (service names, port mappings present) is a Tier-1, no-docker-needed check (`compose_file_is_valid_and_matches_declared_services`-shaped, reused technique). AC6's own real leg lives in the new `m11-acceptance-gate` `workflow_dispatch`-only CI job (Deliverables, §N), needing docker, blocked on both named gaps, exactly as its own first meaningfully-green run is a **later** milestone-acceptance signal, never a condition of this blueprint's own Tier-1 Done state.

### §K — AC7: the manual leg (CROSS-D25), documented, honestly not yet executable to a pass

`docs/MANUAL-VERIFICATION-M11-B07.md` (new, Deliverables) documents the real procedure: obtain a legally owned, pinned-version (CROSS-D6, Bedrock 26.44/protocol 2168) Bedrock game client on a physical or emulated device; point it at a real, `crossplay`-enabled Rusty Clanker server (`M0`–`M7`'s own full acceptance already green, per CROSS-D22's own dependency line); attempt to join; record the outcome. **This procedure is real and complete as written, but — restated honestly, mirroring `M9-B07`/`M10-B06`'s own identical "correct, expected, honest fail" framing for their own comparably gated manual passes — not executable to a real, evidence-backed pass today**: a genuine Bedrock client, exactly like `BedrockBot` (§F), will complete the full connection lifecycle and then receive `UnavailableBedrockTranslator`'s own disconnect message, never rendering a world. The document is authored in full regardless, per PLAN-D5's own "every acceptance criterion is either automated... or a documented, reproducible manual procedure with a pass/fail threshold" mandate — a criterion that cannot currently pass is still a criterion, and its procedure still belongs on record, ready the moment §A gap 1 closes.

### §L — The Java-only regression rerun, extending `M7-B09`'s criterion-4 pattern

`M7-B09` §H, restated in full as this blueprint's own direct precedent: that blueprint's own AC4 split into **AC4a** (a new, harness-owned runtime-level inertness check — §E.1's own direct model) and **AC4b** (*"shells out to `cargo run -p xtask -- m6-report`... with no `[cluster]` config. `M6ReportResult`'s own `TierResult` is embedded, unmodified, as this blueprint's own `M7CompletionReport.ac4.m6_regression` field. No new evaluation logic exists here; this is purely orchestration"*) — the *immediately-preceding milestone's own full acceptance report, re-invoked unmodified against the new milestone's build with the new milestone's own feature disabled*.

**This blueprint's own justified extension of that pattern, restated precisely why it applies here even though CROSS-D26's own text only names a narrower pair of checks (no socket, no tick-time regression):** the same class of claim — *"this optional subsystem ships in every binary but costs nothing, and changes nothing else, unless configured on"* — deserves the same depth of proof CROSS-D26's own rationale text already claims for itself (*"the config-gated-optional-subsystem shape is now proven twice (cluster, crossplay) by the same mechanism"*), and `M7-B09`'s own AC4b already establishes that "twice" is measured by re-running the **full** predecessor acceptance report, not merely its own narrower compile/config-inertness slice. Because CROSS-D22 fixes M11's own dependency line at `M0`–`M7` (never `M8`–`M10`), the correct predecessor to re-invoke is `M7`'s own completion report — the most recent, most comprehensive milestone this project's roadmap actually requires M11 to sit on top of:

```rust
// xtask/src/m11_report.rs (continued)

/// Shells out to `cargo run -p xtask -- m7-report --criterion 4 --server-bin <server_bin>` (the
/// one leg of M7's own report that needs no docker, restated from M7-B09 §I's own tier table) —
/// M7's own already-real AC4 (monolithic no-regression) re-run against the *same*
/// crossplay-compiled-in-but-disabled binary this blueprint's own AC1 leg (§E) also benchmarks.
/// M7's own `Ac4Report` is embedded, unmodified, into this blueprint's own `RegressionReport`.
/// No new evaluation logic — purely orchestration, mirroring M7-B09 §H.1's own identical framing.
pub fn run_regression_rerun(server_bin: &std::path::Path) -> RegressionReport;

#[derive(serde::Serialize)]
pub struct RegressionReport {
    pub m7_report_path: String, // "target/verify/m7-acceptance.json"
    /// `None` if that file is absent/unparseable after the re-invocation — never assumed `pass`
    /// by omission (mirroring M10-B06's own identical `read_m9_status` discipline).
    pub m7_ac4_status: Option<tier_result::Status>,
}
```

### §M — The M11 completion report and the Roadmap Completion rollup

```rust
// xtask/src/m11_report.rs (continued)

pub const OUT_PATH: &str = "target/verify/m11-acceptance.json";

macro_rules! ac_report { ($name:ident) => {
    #[derive(serde::Serialize)]
    pub struct $name { #[serde(flatten)] pub automated: tier_result::TierResult }
} }
ac_report!(Ac1Report); ac_report!(Ac2Report); ac_report!(Ac3Report);
ac_report!(Ac4Report); ac_report!(Ac5Report); ac_report!(Ac6Report); ac_report!(Ac7Report);
// (Deliverables gives the real, non-macro-expanded field lists per AC — the macro above is
// this document's own compact restatement of a uniform shape, never a literal implementation
// requirement; the implementer may write each struct out by hand exactly as every prior
// M<n>ReportResult in this corpus already does, per that lineage's own established convention.)

pub struct M11ReportArgs {
    pub out_dir: std::path::PathBuf,
    pub compose: bool,
    pub regression_server_bin: Option<std::path::PathBuf>,
    pub zero_cost_benchmark: bool,
    pub manual_evidence: Option<std::path::PathBuf>,
    pub roadmap: bool,
}

/// Runs every Tier-1-provable leg for real, folds in `run_regression_rerun`/
/// `run_zero_cost_benchmark` when their own prerequisite flags are supplied, fails every
/// still-gated case closed with its own exact §A-citing message, writes `OUT_PATH`, returns the
/// matching exit code.
pub fn run(args: &M11ReportArgs) -> std::process::ExitCode;

#[derive(serde::Serialize)]
pub struct M11ReportResult {
    pub ac1: Ac1Report, pub ac2: Ac2Report, pub ac3: Ac3Report, pub ac4: Ac4Report,
    pub ac5: Ac5Report, pub ac6: Ac6Report, pub ac7: Ac7Report,
    pub regression: RegressionReport,
    pub roadmap: RoadmapCompletionGate,
    /// `Status::Pass` iff every one of ac1..ac7's own `automated.status` is `Pass` — mirrors
    /// `M7CompletionReport.overall`'s own "wraps several TierResult-shaped sections" rule.
    pub overall: tier_result::Status,
}
```

**§M.1 — The Roadmap Completion rollup**, extending `M10-B06` §Context 11a's own `Phase2Gate`/`PHASE2_NOTE`/`read_m9_status` pattern from a two-milestone gate to the full `M0`–`M11` sequence — the first machine-readable artifact in this corpus stating, explicitly, that the roadmap's own final node has been reached:

```rust
// xtask/src/m11_report.rs (continued)

#[derive(serde::Serialize)]
pub struct MilestoneReportRef {
    pub milestone: &'static str,      // "M1".."M11"
    pub report_path: String,
    /// `None` if the file is absent or lacks a `status`/`overall` field at the expected,
    /// `M<n>ReportResult`-shaped path — a genuinely missing/malformed report is honestly `None`,
    /// never assumed `pass` by omission (M10-B06's own established discipline, restated).
    pub status: Option<tier_result::Status>,
}

#[derive(serde::Serialize)]
pub struct RoadmapCompletionGate {
    /// One entry per `M1`..`M11` (M0 carries no dedicated milestone report — see `m0_note`).
    pub milestone_reports: Vec<MilestoneReportRef>,
    pub m0_note: &'static str,
    /// `true` iff every entry in `milestone_reports` is `Some(Status::Pass)` — purely
    /// informational, never gates this report's own `overall` field (mirroring `Phase2Gate`'s
    /// own identical non-gating stance).
    pub roadmap_complete: bool,
    pub note: &'static str,
}

pub const M0_NOTE: &str = "M0 carries no dedicated m0-acceptance.json: M0-B08 is the \
    verification-wiring blueprint that DEFINES target/verify/<tier>.json (TierResult, the exact \
    shape every later milestone's own m<n>-report reuses) as its own contribution, rather than \
    producing a milestone-specific report of its own kind. M0's own completion is therefore \
    proven by a green Tier-1 CI run against its own changeset — the identical signal every later \
    milestone's own Tier-1 leg already depends on transitively — never by a missing file here.";

pub const ROADMAP_NOTE: &str = "Per 11-roadmap-milestones.md's own PLAN-D2 sequence (M0 Engine \
    Skeleton through M10 Client Feature Parity) plus CROSS-D22's own M11 Bedrock Cross-Play \
    appendix (dependent on M0-M7, independent of M8-M10), M11 is the roadmap's own final node. \
    This report's roadmap_complete field is the first machine-readable statement, anywhere in \
    this corpus, that every milestone M0-M11 this project's planning documents define has its own \
    completion-report lineage: M0-B08 (the mechanism itself), M1-B06 through M10-B06 (one \
    m<n>-acceptance.json apiece), and this blueprint's own m11-acceptance.json. \
    roadmap_complete is purely informational: it restates PLAN-D5's own completion semantics as a \
    fact, and never itself gates this report's own seven AC sections above.";

/// Pure: reads each of `target/verify/m1-acceptance.json`..`m10-acceptance.json` (whichever
/// exist on disk relative to `out_dir`'s own parent) plus this run's own freshly-written
/// `m11-acceptance.json`, extracting a top-level `status` field (the `#[serde(flatten)]`-wrapped
/// `TierResult`'s own field, present in every m1..m6/m8..m11 report) OR, failing that, an
/// `overall` field (m7's own non-flattened `M7CompletionReport` shape, restated as the one named
/// exception this function checks second) — a plain `serde_json::Value` field lookup, never a
/// full typed deserialization, the identical "file-boundary read, never a type-level dependency"
/// discipline `M10-B06`'s own `read_m9_status` already establishes, extended to ten files instead
/// of one.
pub fn build_roadmap_gate(reports_dir: &std::path::Path) -> RoadmapCompletionGate;
```

**The mandatory self-test, `missing_milestone_report_fails_closed`**: `build_roadmap_gate` against a temp directory containing only `m1-acceptance.json`..`m9-acceptance.json` (`m10-acceptance.json` deliberately absent) — asserts `milestone_reports`'s own `M10` entry has `status: None` (never a fabricated `Pass`), and `roadmap_complete == false`.

### §N — CI tier placement

Every real, hermetic leg this blueprint builds — `BedrockBot`'s own login-lifecycle proof (§F), the mixed-session evaluator and its self-test (§G), the world-state comparator and its self-test (§H), the tier-conformance cross-check and its self-test (§I), the inertness scan and the roadmap-gate self-test (§E/§M) — needs no docker, no real pinned-version Bedrock client, no multi-hour wait, and completes well under Tier 1's 10-minute budget (TEST-D37): a handful of `ManagedServer` subprocess spawns (each independently bounded, mirroring every established per-test server lifecycle in this corpus) and a large number of in-process pure-function calls. All of it runs inside the already-existing `gates`/`guardrails` Tier-1 job alongside every prior milestone's own content — **no new Tier-1 job is added by this blueprint.** The `crossplay_zero_cost` criterion benchmark (§E.2) is **not** Tier-1-blocking (a `cargo bench` run, even a short one, does not belong on the fast per-PR path per TEST-D29's own established placement for every other benchmark in this corpus) — it runs as part of the new `m11-acceptance-gate` job below, `workflow_dispatch`-only, alongside the cluster-mode leg. Two new, narrow additions to `.github/workflows/ci.yml` (Deliverables): (1) `on.workflow_dispatch.inputs.job`'s choice list gains `m11-acceptance` as a fifth option (joining `reference-host-gate`/`release`/`compose-topology-gate`/`m7-acceptance` already there, per `M6-B06` §G.1's own reconciliation pattern, already correctly applied by every job since); (2) one new job, `m11-acceptance-gate`, gated on `inputs.job == 'm11-acceptance'`, bringing up `docker-compose.cluster-test.yml` + `docker-compose.m7-acceptance.yml` + `docker-compose.m11-acceptance.yml` together, running `cargo run -p xtask -- m11-report --out-dir target/verify --compose --regression-server-bin <built-binary> --zero-cost-benchmark`, uploading `target/verify/m11-acceptance.json` as a workflow artifact regardless of outcome, and tearing the topology down — mirroring `M7-B09`'s own `m7-acceptance-gate` job shape exactly. `docs/MANUAL-VERIFICATION-M11-B07.md`'s own Tier-3 pass is executed and recorded manually, the same non-CI status every prior manual-verification document in this corpus carries.

## Deliverables

### `crates/testing/paritybot/Cargo.toml` (modify — three new path dependencies, already-workspace-pinned)

```toml
[dependencies]
# ... every existing line unchanged (azalea, tokio, serde, serde_json, ron, thiserror, windows) ...
rc-bedrock-raknet   = { path = "../../bedrock-raknet" }
rc-bedrock-protocol = { path = "../../bedrock-protocol" }
rc-bedrock-auth     = { path = "../../bedrock-auth" }
p384                = { workspace = true, features = ["ecdh", "ecdsa", "pkcs8"] }
```

### `crates/testing/paritybot/src/lib.rs` (modify — one new `pub mod` line, additive)

```rust
pub mod bedrock_bot;
```

### `crates/testing/paritybot/src/bedrock_bot/{mod.rs, chain_signing.rs}` (new)

Per Context §D, full signatures given verbatim above: `BotIdentity`, `BedrockBotError`, `BedrockBot`, `LoginObservation`, `CrossEditionScenario`, `Edition`, `CROSS_EDITION_OBSERVATION_WINDOW_TICKS`, `build_self_signed_claim`, `build_offline_chain`, `build_offline_client_data_token`.

### `crates/testing/paritybot/scenarios/crossplay/{java_places_bedrock_observes.ron, bedrock_places_java_observes.ron}` (new)

Two worked examples of `CrossEditionScenario` (Context §G.1), `actor: Java`/`actor: Bedrock` respectively, otherwise identical fixed position/block/window — the literal "both-simultaneously" pairing realized as two scenario files, per §G.1's own stated resolution.

### `xtask/src/m11_report.rs` (new)

Per Context §E–§M, full signatures given verbatim above: `scan_for_bedrock_targets`, `BEDROCK_TARGET_MARKERS`, `evaluate_inertness`, `PortProbeResult`, `compare_zero_cost_estimates`, `ZeroCostRegression`, `CrossEditionEvent`, `MixedSessionReport`, `evaluate_mixed_session`, `AC2_SPAWN_GATE_MESSAGE`, `hash_block_entities`, `BlockEntitySnapshot`, `WorldStateHash`, `hash_world_state`, `verify_tier_conformance`, `ObservedTierBehavior`, `M11ReportError`, `run_regression_rerun`, `RegressionReport`, `OUT_PATH`, `Ac1Report`..`Ac7Report`, `M11ReportArgs`, `run`, `M11ReportResult`, `MilestoneReportRef`, `RoadmapCompletionGate`, `M0_NOTE`, `ROADMAP_NOTE`, `build_roadmap_gate`, `run_zero_cost_benchmark`, `run_cluster_leg`.

### `crates/server/benches/crossplay_zero_cost.rs` (new)

Per Context §E.2: a `criterion` benchmark driving `ServerComposition`'s own already-real, library-constructible synchronous tick loop over a small, fixed, empty-world scenario for `ZERO_COST_BENCH_TICKS = 200` ticks, referencing no Bedrock type, compiled and run once per feature set per the two `cargo bench` invocations Context §E.2 gives.

### `xtask/src/lib.rs` (modify — one new `pub mod` line, additive)

```rust
pub mod m11_report;
```

### `xtask/src/main.rs` (modify — one new `Command::M11Report` variant, additive)

`Command::M11Report { #[arg(long)] out_dir: PathBuf, #[arg(long)] compose: bool, #[arg(long)] regression_server_bin: Option<PathBuf>, #[arg(long)] zero_cost_benchmark: bool, #[arg(long)] manual_evidence: Option<PathBuf>, #[arg(long)] roadmap: bool }`, dispatched to `m11_report::run` — the same additive-variant shape every prior blueprint's own `Command` extension already established.

### `.github/workflows/ci.yml` (modify — two additive changes, per Context §N)

1. `on.workflow_dispatch.inputs.job`'s choice list gains `m11-acceptance`.
2. One new job, `m11-acceptance-gate` (`workflow_dispatch`-only, `if: inputs.job == 'm11-acceptance'`).

### `deploy/cluster/docker-compose.m11-acceptance.yml` (new, non-code)

Per Context §J: a third, co-layered compose override adding one published Bedrock UDP port and one `[crossplay]` config block to each of `M7-B09`'s own already-declared `proxy-*` services — every existing base/overlay file unchanged.

### `docs/MANUAL-VERIFICATION-M11-B07.md` (new)

Per Context §K: the real, CROSS-D25-mandated manual procedure, honestly annotated as not yet executable to a pass.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, binding):** every file listed below, plus every function body in `crates/testing/paritybot/src/bedrock_bot/{mod.rs, chain_signing.rs}` and `xtask/src/m11_report.rs` replaced by `todo!()` (every struct field, enum variant, derive, and public signature stays exactly as Context/Deliverables fix it), are committed first. The implementation changeset fills `todo!()` bodies, writes `crates/server/benches/crossplay_zero_cost.rs`, the two `.github/workflows/ci.yml` edits, `deploy/cluster/docker-compose.m11-acceptance.yml`, and `docs/MANUAL-VERIFICATION-M11-B07.md`; it must not modify any test file listed below, and must not touch any pre-existing `M11-B01`–`M11-B06` test file. **`crates/testing/paritybot/**` needs no new `PROTECTED_PATHS` row** — `M1-B06`'s own already-committed `ProtectedPath { pattern: "crates/testing/paritybot/**", .. }` already covers this blueprint's own new `src/bedrock_bot/`, `scenarios/crossplay/`, and `tests/` additions in full, proven by `path_guard_already_covers_this_blueprints_new_paths` below.

### `crates/testing/paritybot/tests/bedrock_bot_login.rs` (new)

1. `bedrock_bot_completes_full_lifecycle_then_receives_honest_unavailable_disconnect` — per Context §F, against a real `--offline`, `crossplay`-enabled `rusty-clanker-server` subprocess (`ManagedServer`, `M3-B08`).
2. `bedrock_bot_offline_chain_is_accepted_by_a_real_server` — a narrower, isolated case: `BedrockBot::complete_login` reaches at least `ClientToServerHandshake` sent (encryption installed) before any error, against the identical real subprocess — proves the chain/handshake half in isolation from the resource-pack/disconnect half above.
3. `bedrock_bot_rejects_a_tampered_server_handshake_token` — a fixture-injected, deliberately malformed `ServerToClientHandshakePacket` (hand-constructed, never from a real server) fed directly to `BedrockBot`'s own internal handshake-processing step (a `pub(crate)` test seam, implementer's own naming freedom) — `Err(BedrockBotError::Login{..})`, never a panic.

### `crates/testing/paritybot/tests/bedrock_bot_wire_proxy.rs` (new)

`movement_and_block_action_and_chat_construct_well_formed_packets` — `send_movement`/`send_block_action`/`send_chat` against an in-test loopback peer (a bare `tokio::net::UdpSocket`, mirroring `M11-B01`'s own `loopback_two_socket_integration` technique, never a real server), asserting each resulting sub-packet decodes via `rc_bedrock_protocol::BedrockPacket::decode` into the exact expected `PlayerAuthInputPacket`/`TextPacket` value.

### `xtask/tests/m11_report.rs` (new)

1. `inertness_scan_finds_no_bedrock_targets_in_a_clean_log` / `leaked_bedrock_thread_fails_inertness` **(mandatory self-test)** — the second, a synthetic log line `"INFO rc_bedrock_raknet::socket: listening"` injected into an otherwise-clean fixture; `scan_for_bedrock_targets` returns `Some(_)`, `evaluate_inertness`'s own resulting `TierResult.status == Status::Fail`.
2. `zero_cost_benchmark_within_tolerance_passes` / `zero_cost_benchmark_regression_over_five_percent_fails` — `compare_zero_cost_estimates(100.0, 96.5)` → `Ok(_)` (a 3.6% delta, tolerance's own reused ≥5% threshold, §E.2); `compare_zero_cost_estimates(100.0, 90.0)` → `Err(ZeroCostRegression{delta_percent}) if delta_percent >= 5.0`.
3. `evaluate_mixed_session_confirms_a_real_cross_edition_pair` / `bedrock_invisible_to_java_fails_the_mixed_session_evaluator` **(mandatory self-test)** — the first, a fixture event log with a matching `ActionSent`/`WorldChangeObserved` pair inside the scenario's own window, `passed == true`; the second, per Context §G.2's own exact fixture, `passed == false`.
4. `hash_world_state_is_deterministic_and_order_independent_for_block_entities` / `divergent_world_state_fails_the_cross_edition_comparator` **(mandatory self-test)** — the first, two block-entity lists in different insertion order but identical content hash identically; the second, per Context §H.2, `hash_world_state`'s `block_states` field differs across one deliberately-perturbed cell.
5. `verify_tier_conformance_accepts_a_consistent_table` / `translation_regression_fails_the_tier_conformance_suite` **(mandatory self-test)** — the second, per Context §I's own exact fixture (a claimed `Tier::Parity` row whose observed behavior is `Tier::Degraded`), `Status::Fail`.
6. `build_roadmap_gate_reports_complete_when_every_report_passes` / `missing_milestone_report_fails_closed` **(mandatory self-test)** — the second, per Context §M's own exact fixture (`m10-acceptance.json` absent), `M10` entry `status: None`, `roadmap_complete == false`.
7. `run_regression_rerun_embeds_m7s_ac4_unmodified` — a fixture `target/verify/m7-acceptance.json` (hand-authored, matching `M7CompletionReport`'s own real shape) fed via a stubbed subprocess-invocation seam (implementer's own test-injection technique, mirroring `M7-B09`'s own established pattern for its own `m6-report` re-invocation test); `RegressionReport.m7_ac4_status` matches the fixture's own `ac4.automated.status` exactly.
8. `no_manual_evidence_gates_every_live_confirmation_case` — `m11_report::run` with every gate-dependent flag absent; asserts `ac2.spawn_into_terrain`, `ac3.live_round_trip`, `ac4.live_world_state_comparison`, and `ac6.automated` (the cluster leg as a whole) all report `Status::Fail` with their own exact §A-citing message, and every other case (`ac1`, `ac5`, `regression` once `--regression-server-bin` is supplied) is real and, given a correctly-behaving fixture/subprocess environment, `Status::Pass`.
9. `parses_m11_report_cli_flags` — `Cli::try_parse_from(["xtask", "m11-report", "--out-dir", "target/verify"])` matches `Command::M11Report { compose: false, .. }`; a second case with every flag present matches accordingly.
10. `path_guard_already_covers_this_blueprints_new_paths` — `xtask::path_guard::check_paths` against this blueprint's own full changed-file list (every new `crates/testing/paritybot/**` path plus `xtask/**`, `crates/server/benches/**`, `.github/workflows/ci.yml`, `deploy/cluster/**`, `docs/**`) under `ChangesetType::TestAuthoring`/`Implementation` as appropriate returns zero violations for the already-covered `paritybot/**` paths — proving no new `PROTECTED_PATHS` row is needed there, per Context §D's own claim.

## Implementation steps

1. **`crates/testing/paritybot/Cargo.toml`.** The three new path dependencies plus `p384`, exactly as Deliverables. Observable: `cargo build -p rc-paritybot` compiles with every new module a stub.
2. **`crates/testing/paritybot/src/bedrock_bot/chain_signing.rs`.** Implement `build_self_signed_claim`/`build_offline_chain`/`build_offline_client_data_token` per Context §D.1, reusing `base64`/`serde_json`/`p384::ecdsa` directly, mirroring `M11-B03`'s own already-committed `make_claim` test helper's own algorithm exactly (restated, not imported).
3. **`crates/testing/paritybot/src/bedrock_bot/mod.rs`.** Implement `BedrockBot::{connect, complete_login, send_movement, send_block_action, send_chat, recv_raw, disconnect}` per `M11-B06` §E's own algorithm, client side — `complete_login` reusing `rc_bedrock_auth::handshake::{ServerEcdhKeyPair, generate_salt, BedrockAeadEncryptor, BedrockAeadDecryptor}` directly per Context §D.1's own reuse argument. Observable: `bedrock_bot_login.rs`'s three cases pass; `bedrock_bot_wire_proxy.rs` passes.
4. **`crates/testing/paritybot/scenarios/crossplay/*.ron`.** Author the two worked examples exactly as Deliverables/§G.1 specify.
5. **`xtask/src/m11_report.rs`.** Implement every function per Context §E–§M, in the order those sections appear. Observable: `xtask/tests/m11_report.rs`'s ten cases pass incrementally as each section's own implementation lands.
6. **`xtask/src/lib.rs`, `xtask/src/main.rs`.** Add the module declaration and `Command::M11Report` variant. Observable: `cargo run -p xtask -- m11-report --help` succeeds.
7. **`crates/server/benches/crossplay_zero_cost.rs`.** Implement the tick-loop benchmark per Context §E.2, referencing no Bedrock type. Observable: both `cargo bench --no-run` invocations (Done-when) succeed.
8. **`.github/workflows/ci.yml`.** Add the `inputs.job` option and the `m11-acceptance-gate` job per Context §N. Observable: a YAML-parse check confirms both edits (no self-hosted runner exists yet to dispatch to for a first real run, mirroring `M7-B09`'s own identical framing for its own new CI wiring).
9. **`deploy/cluster/docker-compose.m11-acceptance.yml`.** Per Context §J. Observable: the structural, no-docker-needed validity check (Deliverables) passes.
10. **`docs/MANUAL-VERIFICATION-M11-B07.md`.** Write per Context §K's content.
11. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test`, `-- path-guard` — all five exit 0.
12. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** No test file, test case, or assertion in Acceptance tests may be added, removed, renamed, or weakened by the implementation changeset — in particular the five mandatory self-tests, each pinning a load-bearing gate against silent drift.

(b) **No new external dependencies beyond what Deliverables names.** `rc-bedrock-raknet`/`rc-bedrock-protocol`/`rc-bedrock-auth`/`p384` are every crate this blueprint's own `rc-paritybot` extension needs, all already `[workspace.dependencies]`-pinned by `M11-B01`/`M11-B02`/`M11-B03` — zero new workspace pins. `criterion` (`crates/server/benches/`) is already `[workspace.dependencies]`-pinned (TEST-D29). Do not add `gophertunnel`, any Node.js/Go subprocess-bridging mechanism, or any third-party Bedrock bot library (CROSS-D23's own explicit non-goal, Context §C).

(c) **No Mojang or third-party reimplementation code.** Every algorithm this blueprint's `BedrockBot` uses is restated from `M11-B01`/`M11-B02`/`M11-B03`'s own already-source-verified content; `chain_signing.rs`'s own JWT-building logic is this blueprint's own original re-authoring of `M11-B03`'s own already-approved test-helper pattern, never third-party code.

(d) **Dependency-graph discipline.** `rc-paritybot` gains exactly the three named path dependencies; nothing under `crates/bedrock-*/src/` is touched by this blueprint at all. `rc-bedrock-raknet`/`rc-bedrock-protocol`/`rc-bedrock-auth`'s own CROSS-D5 dependency ceilings are unaffected — `rc-paritybot` is a testing crate outside every set `xtask lint-deps` enumerates for the production Bedrock crates.

(e) **Scope boundary — do not implement beyond this blueprint's stated Deliverables.** This blueprint does not implement: a concrete `BedrockTranslator` (§A gap 1, `M11-B06`'s own explicit non-goal, restated as this blueprint's own too); `main.rs`'s `ClusterProxy`/`ClusterNode` real-serving wiring (§A gap 2, `M7-B08`'s own inherited gap); `09-testing-quality.md`'s own general TEST-D7/TEST-D8/TEST-D11 vanilla-differential harness (§A's third, corpus-wide gap — this blueprint's own `CrossEditionScenario` format is a deliberately narrower, RC-vs-RC-only instantiation, never a claim of having built the general one); any change to `rc-bedrock-*`'s own `src/`, `rusty-clanker-server`'s own `src/`, or `rc-proxy`'s own `src/`. Every gated case's `fail` status and this blueprint's own honest, precise message naming the exact missing contract item is the correct, expected Done state until a future sibling blueprint lands, not a defect this blueprint's implementer should "fix" by faking a pass.

(f) **`unsafe` code is forbidden.** Every deliverable in this blueprint is ordinary safe Rust.

(g) **`ManagedServer`'s own per-test process-isolation discipline is binding (restated from every prior harness blueprint in this corpus).** No test file this blueprint adds shares one live server process across multiple `#[test]` functions.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-paritybot -p xtask --all-features
cargo bench -p rusty-clanker-server --no-run --features "monolithic crossplay"
cargo bench -p rusty-clanker-server --no-run --no-default-features --features monolithic
cargo nextest run -p rc-paritybot -p xtask -p rusty-clanker-server -p rc-proxy
cargo test --doc -p rc-paritybot
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- path-guard
cargo run -p xtask -- test
cargo run -p xtask -- m11-report --out-dir target/verify
cargo run -p xtask -- m11-report --out-dir target/verify --roadmap
```

Expected: every command exits 0; the second-to-last command's `target/verify/m11-acceptance.json` reports `ac1`/`ac5` (and `ac2`'s own `lifecycle_completes` case, and `ac3`/`ac4`'s own evaluator/comparator self-test-backed cases) `pass`, and `ac2.spawn_into_terrain`/`ac3.live_round_trip`/`ac4.live_world_state_comparison`/`ac6` `fail` with their own exact, actionable, §A-citing messages — this is this blueprint's own correct, expected Done state until the still-missing composition-root/ECS-adapter/Stage-11-integration blueprint(s), and separately `M7-B08`'s own still-open cluster-integration gap, land, not a defect. The final command's own `roadmap.roadmap_complete` reads `false` in this state (since this run's own `overall` is `Fail` while any gated case remains open) — also correct and expected. CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.

## Interfaces

**Needs from a future composition-root/ECS-adapter blueprint (§A gap 1 — restated a sixth time in this corpus's own M11 lineage, the identical gap `M11-B05`'s own Interfaces section and `M11-B06`'s own Constraints (f) already name):** a concrete `BedrockTranslator` implementation replacing `UnavailableBedrockTranslator`, wired into `ServerComposition`'s real `PlayerSessionSink`/join-time-resolution path. Once satisfied, `Ac2Report.spawn_into_terrain`, `Ac3Report.live_round_trip`, `Ac4Report.live_world_state_comparison`, half of `Ac6Report`'s own gate, and `docs/MANUAL-VERIFICATION-M11-B07.md`'s own procedure all become exercisable without a single test-logic rewrite — every one of this blueprint's own gated cases is written against real, already-committed `M11-B01`–`M11-B06` types, so only the evidence source changes, never the assertion.

**Needs from a future Stage-11-integration blueprint (§A gap 1's own second half, `M11-B05` Interfaces):** the dirty-generation-keyed shared-encode cache wrapping `translate_section`/`translate_entity_*` — needed for the outbound half of AC2/AC3/AC4's own live legs, alongside the composition-root/connection-driver half above.

**Needs from `M7-B08`'s own still-open Context §A items 1/3 (§A gap 2, restated from `M7-B09`):** a concrete, real-network `openraft::RaftNetworkFactory`/`JoinClient`, and `main.rs`'s own `ClusterProxy`/`ClusterNode` real-serving wiring. Once satisfied (independently of gap 1, per this blueprint's own doubled-gate framing, §J), `Ac6Report`'s own real leg becomes exercisable for the login/relay half; both gaps together are needed for a genuine, node-side-translated Bedrock handoff.

**Needs from `09-testing-quality.md`'s own next revision (§A's third gap):** ratification of this blueprint's own `CrossEditionScenario` format (Context §G.1) as a legitimate, narrower-scoped instantiation of TEST-D11's own decision, pending the eventual general TEST-D7/TEST-D8/TEST-D11 vanilla-differential harness no blueprint through `M10` has yet built; and an explicit addition of `crates/*/benches/` directories to TEST-D46's own enumerated protected-path categories (this blueprint's own `crates/server/benches/crossplay_zero_cost.rs` is treated as test-changeset content by this blueprint's own convention, Constraints (a), but is not yet covered by an enumerated `PROTECTED_PATHS` row the way `tests/`/fixture directories already are).

**Provides to `11-roadmap-milestones.md`'s own next revision:** the first machine-readable, PLAN-D5-conformant statement that the roadmap's `M0`–`M11` sequence — every milestone this project's planning corpus defines — has a defined completion signal (`RoadmapCompletionGate`, Context §M.1); that document's own next revision may cite this blueprint's `xtask m11-report --roadmap` invocation as the concrete answer to "how would an agent verify the whole roadmap is done."

**Provides to a future account-linking or NetherNet/WebRTC revision (`15-crossplay.md`'s own Open Questions):** confirmation, via this blueprint's own honest gap-naming discipline (§A), that neither of those two named-but-deferred directions has any bearing on this blueprint's own acceptance surface — both stay entirely out of scope, unaffected.

## Open Questions

- **`RoadmapCompletionGate`'s own eventual "everything green" state is not reachable until both `§A` gaps close and every `M0`–`M10` predecessor's own real leg (docker/reference-hardware-gated legs included) is independently re-verified** — this blueprint's own `roadmap_complete` field is designed to flip to `true` automatically the moment every input file says `Pass`, but that moment is not this blueprint's own to force; a future pass, once the composition-root/ECS-adapter/Stage-11-integration blueprint(s) and `M7-B08`'s own gap both land, should re-run every `m<n>-report` from a clean checkout and confirm the rollup actually flips, closing the loop this blueprint only opens.
- **`crates/*/benches/` is not yet an enumerated `TEST-D46` protected-path category** (Interfaces) — this blueprint treats its own new `crates/server/benches/crossplay_zero_cost.rs` as test-changeset content by convention; a future revision of `09-testing-quality.md` should close this gap explicitly rather than leave every future benchmark-as-acceptance-evidence file to rely on the same convention informally.
- **The exact wire-level shape of a "leaked Bedrock thread" beyond a `tracing`-target substring match** (§E.1) is this blueprint's own necessarily approximate proxy — a Tokio task that never logs anything through `tracing` at all would evade `scan_for_bedrock_targets` entirely; a future revision could strengthen this via `tokio-console`'s own task-naming/introspection surface (already available as a development tool, never a shipped dependency per TEST-D30's own identical "developer-run desktop tool, not bundled" stance for Tracy) if this approximate check is ever found insufficient in practice.
- **Whether a future revision should wrap `gophertunnel` via a subprocess harness to strengthen CROSS-D23's own test independence beyond dogfooding** — restated, unresolved, from `15-crossplay.md`'s own Open Questions, left exactly as open here; this blueprint's own `BedrockBot` is built entirely consistent with either outcome (a future subprocess-bridged bot would be a parallel, independent driver this blueprint's own evaluators — `evaluate_mixed_session`, `hash_world_state`, `verify_tier_conformance` — could consume identically, since none of them are coupled to `BedrockBot`'s own internal implementation, only to the plain event/hash/table types it and any future driver alike would produce).
- **CROSS-D22/CROSS-D23/CROSS-D24/CROSS-D25/CROSS-D26's own text is restated throughout this blueprint field-by-field, but none of the honestly-invented resolutions this blueprint contributes** (the `CrossEditionScenario` format, the `MixedSessionReport`/`WorldStateHash`/`TIER_TABLE`-cross-check evaluator shapes, the doubled `ClusterAndTranslatorIntegrationPending` framing) **is yet reconciled into `15-crossplay.md`'s own CROSS-D decision register** — flagged here for a future revision of that document, mirroring every prior M11 blueprint's own identical "flagged for reconciliation, never presented as an already-ratified decision" discipline.
