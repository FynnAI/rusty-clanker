# M9-B07 — Client Bootstrap Acceptance Harness

| Field | Content |
|---|---|
| ID | M9-B07 |
| Milestone | M9 — Client Bootstrap: Connect & Render a Static World |
| Prerequisites | M9-B03 (client auth/connection — `rc-msa-auth`'s full public surface; `rusty-clanker-client`'s `connection::{ClientConnection, run_client_session, ClientSessionSettings, crypto::*}` and `world::{ClientWorld, PlayerState, PlayerPosition, ClientChunkColumn}` exactly as already committed — this blueprint drives `run_client_session` directly against a REAL `rusty-clanker-server` subprocess, never modifying any B03 file; `docs/MANUAL-VERIFICATION-M9-B03.md` §15's own auth-then-connect procedure, restated and folded into this blueprint's own consolidated manual pass, never duplicated verbatim). M9-B05 (`rc-render`'s `bake::{bake_all, BakedRegistry}`, `section_snapshot::{SectionSnapshot, BiomeColumnGrid, SnapshotProvider, HALO_WIDTH}`, `mesh::MeshWorkerPool` types, `vertex::{Vertex, unpack_pos_and_face_frac, unpack_light_and_ao_tint}` exactly as already committed — this blueprint feeds real, network-received block data into `bake_all`+the mesher via a harness-owned fixture bridge, §Context 3, never a production `SnapshotProvider` implementation). M9-B06 (`crates/client/src/player::{PlayerController, CadenceState, MovementReport, apply_synchronize}` exactly as already committed — this blueprint drives real scripted input through it against a real server). M9-B01/M9-B02/M9-B04 (consulted, not re-touched — `net::{OutboundIntent, NetworkHandle}`, `rc-assets`' `discovery`/`store` surface, `rc-render`'s `device::negotiate_device_requirements`/`renderer::TerrainRenderer`/`camera::Camera`, cited by name for the composition-root contract this blueprint pins in §Context 2, never implemented here). M6-B04 (`xtask::reference_host::{ReferenceHostSpec, ReferenceHostTier, TierId, HostFingerprint, match_tier, is_match, AuthoritativeRunReport, gate, load_spec, validate_spec, probe_host}` — this blueprint's own **additive, backward-compatible** extension, §Context 6, never touches the three existing tiers' committed values or `match_tier`'s existing 11 checks). M0-B08 (`xtask::tier_result::{TierResult, CaseResult, Status, write, write_to, VERIFY_OUT_DIR, exit_code_for}`, `xtask::path_guard`'s `Changeset-Type` trailer convention and `PROTECTED_PATHS` — `xtask/**` (row 7) and `reference-hosts.toml` (M6-B04's row) already cover every path this blueprint's implementation changeset touches). M6-B01/M6-B06/M7-B09/M8-B05 (the established acceptance-harness lineage: `M<n>ReportResult`/`TierResult` via `#[serde(flatten)]`, one `xtask m<n>-report` entry point, per-criterion `CaseResult`s, a pure `build_report` aggregation function, mandatory harness self-tests, and — M8-B05's own precedent specifically — the binding "pin the exact contract a still-missing sibling blueprint must satisfy; prove everything else hermetically against real, locally-buildable artifacts; fail closed" split, applied below to M9's own genuine composition-root gap, §Context 2). M9-B02 (`xtask::content_audit::{Hit, scan, run}` — this blueprint's own **additive** extension, §Context 5, never removes or weakens B02's own committed detectors or its `xtask/tests/content_audit.rs` suite). |
| Implements | `11-roadmap-milestones.md`'s M9 Acceptance Criteria 1–3, verbatim (Context §1) — this blueprint **is** their concrete, agent-executable measurement, per PLAN-D5. ASSET-D2/D3/D6 (the real-account auth pass, restated as this blueprint's own consolidated manual-step list). CLIENT-D28/CLIENT-D30 (prediction/reconciliation and tick/render decoupling, exercised end to end for the first time against a real server). CLIENT-D32/PERF-D63/PERF-D64 (the frame-budget/stable-FPS measurement definition this blueprint pins concretely, §Context 4). ASSET-D24 (the release-artifact content-audit rule, extended to the client binary for the first time — no prior blueprint wires this). TEST-D7-adjacent differential-harness shape (client-vs-real-server, not client-vs-vanilla — restated, §Context 2). TEST-D37 (CI tier placement). TEST-D40 (machine-readable M9 completion report). TEST-D45/D46/D50/D52 (test-first changeset boundary, protected-path coverage, CI-is-authority, verifier re-run). |
| Crates touched | `crates/client/` (`rusty-clanker-client`, additive test-only content: `tests/{m9_leg1_block_placement, m9_leg1_mesh_render_proxy, m9_leg2_position_roundtrip, common/real_server}.rs` — no `src/` change, no `Cargo.toml` change). `xtask` (additive: `src/{m9_report, frame_time, client_release_audit}.rs`; **additive** deltas to already-committed `src/{content_audit, reference_host, path_guard, main}.rs`; one new repo-root data-file delta, `reference-hosts.toml`). `.github/workflows/ci.yml` (one new, additive, `workflow_dispatch`-only job). `docs/MANUAL-VERIFICATION-M9-B07.md` (new). **No production `src/` file outside `xtask` is touched anywhere** — this blueprint authors no `SnapshotProvider` implementation, no `Renderer`-trait wrapper, and no asset-pipeline startup sequence; §Context 2 explains why and pins the exact contract instead. |
| Estimated scope | L — every sub-mechanism below is independently small; the size comes from three legs plus a schema extension, not from any one piece being deep. |

## Goal & Done definition

Wire M9's three acceptance criteria (`11-roadmap-milestones.md`) into one agent-executable, machine-readable measurement, `xtask m9-report`, continuing the exact lineage M6-B01/M6-B06/M7-B09/M8-B05 already established — **built entirely against real, already-committed B03/B05/B06 code and a real, locally-buildable `rusty-clanker-server` subprocess, never a hand-built stub standing in for either.** Concretely: (1) **Leg 1**: a real client-vs-real-server block-state comparison proving B03's chunk decode is correct end to end (§Context 2's genuine gap — no rendering integration exists yet — is pinned as a binding contract, not implemented), plus a real mesh-correctness proxy driving B05's own bake/mesh pipeline against that same real, network-received block data, plus the documented real-account auth pass and a screenshot-capture step honestly framed as non-gating evidence (09's own visual-verification stance, restated); (2) **Leg 2**: a real, scripted position-round-trip session driving B06's prediction/cadence code against a real server, plus a pure, hermetically-tested frame-time/stable-FPS analysis function and the documented reference-host FPS pass this milestone's own acceptance bar requires; (3) **Leg 3**: `xtask content-audit` (B02) extended with JPEG magic-byte detection, a best-effort Mojang-asset-hash cross-check, and model-JSON-signature detection, wired for the first time into a real client release build and a new, `workflow_dispatch`-triggered CI job; (4) M6-B04's reference-host fingerprint schema extended, additively, with a fourth tier and an optional GPU field (§Context 6); (5) CI tier placement and the machine-readable M9 completion report, continuing the established `M<n>ReportResult` shape; (6) three mandatory harness self-tests, each proving a named failure mode this blueprint's own gates are supposed to catch is actually caught.

**The one genuine, honestly-disclosed gap this blueprint depends on and does not implement** (Context §2): no merged blueprint wires a real `rc_render::renderer::TerrainRenderer` into `rusty-clanker-client`'s `Shell`/`Renderer` seam, negotiates real device features, or implements a production `SnapshotProvider` translating `ClientWorld` into `SectionSnapshot`s — M9-B04 §Context 3/Interfaces and M9-B05 §Interfaces both name this gap explicitly and leave it open; M9-B06 §Interfaces speculates it might land in "M9-B06 or a dedicated M9-B07" and, having itself not closed it, leaves the question open. **This blueprint's own resolution: it is not this blueprint's job either.** A harness blueprint's role in this corpus's own established lineage (M6-B01 §B, M5-B10 §A.3, M6-B04's own explicit non-scope, M8-B05 §A) is to pin the missing contract precisely, build and prove everything else hermetically against real, already-real artifacts, and fail closed — never to quietly absorb a composition-root implementation task into a harness blueprint's own scope. What genuinely cannot be proven without that still-future integration blueprint — a real window showing real, textured terrain, and a real frame-rate/10-minute-session measurement — is named precisely (Context §2's binding contract) and reported, correctly and honestly, as `fail` with an actionable message until that blueprint lands.

Done when:

- [ ] `cargo build -p rusty-clanker-client -p xtask --all-features` succeeds with zero warnings, on both `ubuntu-24.04` and `windows-2025`.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rusty-clanker-client -p xtask`; every M9-B02/M9-B03/M9-B05/M9-B06 test that already exists continues to pass unmodified.
- [ ] `m9_leg1_block_placement.rs`'s `client_world_matches_real_server_bedrock_floor` passes: a real `rusty-clanker-server` subprocess (`--offline`, a fresh `--world-dir`) is started, a real `run_client_session` connects and completes its initial chunk-load sequence, and every sampled position at world floor `y = -64` within the loaded area reads back the real `minecraft:bedrock` block-state id from `ClientWorld` — proven against the real, generated registry id, never a hardcoded guess.
- [ ] `m9_leg1_mesh_render_proxy.rs`'s case passes: the same real, received block data is bridged into `rc_render::bake::bake_all` + a mesh job, and the resulting `MeshData` has the expected non-zero opaque face count with correctly-resolved `material`/`light_and_ao` fields for the known bedrock/stone pattern.
- [ ] `m9_leg2_position_roundtrip.rs`'s case passes: a scripted `InputSnapshot` sequence driven through a real `PlayerController` against a real server, for `M9_AUTOMATED_SESSION_TICKS` ticks, produces continuous, correctly-cadenced movement packets with zero desync between the client's predicted position and the server's own authoritative echo.
- [ ] `frame_time.rs`'s `analyze_frame_times` passes both its "stable" and "unstable" (stall-injected) fixtures — the mandatory Leg-2 self-test.
- [ ] `content_audit.rs`'s extended detectors pass their own fixtures, including the mandatory Leg-3 self-test: a deliberately embedded PNG fixture in a synthetic release-archive tree fails the scan.
- [ ] `m9_leg1_block_placement.rs`'s mandatory Leg-1 self-test (`divergent_block_fails_the_comparison`) passes: the exact comparison function used above, fed a deliberately mutated `ClientWorld` copy, reports the specific mismatched position as a failure.
- [ ] `cargo run -p xtask -- m9-report --out-dir <dir>` (no `--manual-evidence`) runs Legs 1a/1b/2a/3 for real and writes `target/verify/m9-acceptance.json`; Legs 1c (auth + screenshot) and 2b (reference-GPU FPS session) report `fail` with the exact, actionable §Context 2-citing message — this is this blueprint's own correct, expected Done state until a future client-integration blueprint lands, not a defect.
- [ ] `cargo run -p xtask -- m9-report --out-dir <dir> --manual-evidence <fixture>.json` (a hand-built fixture, not a real session) reports Legs 1c/2b `pass` when the fixture's fingerprint matches the `m9-client-reference` tier and every recorded value clears the stable-FPS/zero-crash bar, and `fail` when it does not.
- [ ] `cargo run -p xtask -- host-fingerprint --tier m9-client-reference --gpu-info <fixture>.json` completes without panicking and writes a schema-valid `target/verify/host-fingerprint.json`, extending M6-B04's own verb.
- [ ] `cargo run -p xtask -- path-guard` exits 0 against this blueprint's own changesets, correctly labeled per Constraints.
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rusty-clanker-client -p xtask` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`, `path-guard`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50). The new `client-release-audit` CI job (Deliverables) is `workflow_dispatch`-only, mirroring `reference-host-gate`'s own established shape, and is not part of the required Tier-1 status-check set.

## Context (self-contained)

### 1. M9's three acceptance criteria, verbatim, and this blueprint's own precise reading of each

From `11-roadmap-milestones.md`, quoted in full:

1. *"The native client connects to a Rusty Clanker server (an `M1`–`M6`-feature-complete build), authenticates via a real Microsoft/Mojang account, and renders a generated world's terrain correctly textured from the player's legally-owned local assets, with block placement matching server state 1:1."*
2. *"Camera movement and basic input round-trip to the server (movement sent, server-authoritative position reflected back) at a stable, documented frame rate on a reference GPU for a continuous 10-minute session with zero crashes."*
3. *"A release-artifact content audit confirms zero PNG/OGG/model/Mojang-derived binary assets anywhere in the client binary or its build archive — every visual/audio asset present at runtime was loaded from the player's own local installation, never shipped."*

**§1.1 — AC1, partitioned.** Four sub-parts, each independently gated: **AC1a (block placement 1:1)** — mechanically defined as "for a fixed, known set of world positions, the client's `ClientWorld` reports the identical block-state id the server itself holds at that position" — **this blueprint's own real, new proof** (§Context 2's "known pattern," below), since no earlier M9 blueprint runs a client against a real, standalone server subprocess at all (B03's own tests run only against its in-process `fake_server.rs`, by design — its own Prerequisites list states this explicitly). **AC1b (renders... correctly textured)** — split, per §Context 2's gap, into an **automated proxy** (mesh/world-state assertions against the real received block data, driven through B05's real bake/mesh pipeline — this blueprint's own new proof) and the **actual pixel-correct render**, which needs the still-missing integration blueprint and is therefore a documented, evidence-gated manual pass, never silently claimed automated. **AC1c (authenticates via a real Microsoft/Mojang account)** — the one deliberately manual step this whole corpus's verification loop ever requires (TEST-D41/ASSET-D3's own precedent, M1's own real-account pass), restated as this blueprint's own consolidated step list, folding in B03 §15's already-committed content rather than duplicating it. **AC1d** is not a separate criterion — it is AC1c's own natural continuation ("everything after auth is automated," this blueprint's own binding reading): the automated tests below all run in `--offline` mode (ASSET-D1/NET-D6's own "offline-mode is retained for local/LAN testing... never the default" stance, and B03's own Done-bar's binding "zero test using a real Microsoft account credential") precisely so the auth-chain's own correctness (already B03's job, already tested there) is never re-proven here, and no CI job anywhere in this corpus ever needs a live Microsoft credential.

**§1.2 — AC2, partitioned.** **AC2a (position round-trip)** — fully automated, real server, real B06 code (§Context 4). **AC2b (stable, documented frame rate on a reference GPU, continuous 10 minutes, zero crashes)** — needs a real rendering client (§Context 2's gap); this blueprint supplies the **measurement definition** (a pure, hermetically-tested function, §Context 4) and the **evidence contract** (§Context 6's `ManualEvidence`, gated through M6-B04's own `AuthoritativeRunReport` mechanism), not a fake automated pass.

**§1.3 — AC3.** Fully automatable today — no rendering integration is needed to scan a built binary's bytes. §Context 5.

### 2. The one genuine gap, its binding contract, and why the rest of this blueprint does not wait on it

`07-client-architecture.md`'s CLIENT-D3/D26 and this milestone's own four prior blueprints together fully specify: the client shell and its `Renderer`/`InputConsumer`/`ClientSimulation` seams (B01), the local asset pipeline (B02), auth and the wire-protocol connection plus a real client-side chunk store (B03), the wgpu rendering foundation (B04), the blockstate/model interpreter and chunk mesher (B05), and camera/movement prediction (B06). What none of the six implements — each says so explicitly, cited here rather than re-derived — is the **composition-root glue** wiring them into one running, rendering process: M9-B04 §Context 3 names, in full, "a thin wrapper type inside `rusty-clanker-client`... holding a `TerrainRenderer` and implementing M9-B01's `Renderer` trait," "replace M9-B01's `GraphicsContext::new` stub... with `negotiate_device_requirements`'s output," and "wiring an actual `tracing_tracy::TracyLayer` subscriber" as a **future** blueprint's job, explicitly not itself. M9-B05 §Interfaces names, as its own **not-yet-written** dependency, "a real `SnapshotProvider` implementation translating the client's own chunk storage... into `SectionSnapshot` values," "a real call site driving `MeshWorkerPool::mark_dirty`," "a real per-frame loop draining `try_recv` into `TerrainRenderer::submit_section_mesh`." M9-B06 §Interfaces provides `camera_params_and_update` as the ready-to-consume call a future blueprint wires into a real render step, and speculates — without closing it — that this future blueprint is "M9-B06 or a dedicated M9-B07."

**This blueprint's own binding resolution, restated plainly:** it is neither. A harness blueprint's job, per this corpus's own repeatedly-established lineage (M6-B01 §B: "this blueprint's own task is to wire M6's *acceptance criteria*, not to close that gap"; M8-B05 §A: "this blueprint's own task is to wire M8's *acceptance criteria*, not to close that gap... the identical division of labor M6-B01/M6-B06 already drew"), is to state the missing contract precisely and build everything provable without it. This blueprint does exactly that, and no more.

**The binding contract, restated in full, on whichever future client-integration blueprint closes it** (consolidating M9-B04 §Context 3/Interfaces and M9-B05 §Interfaces into one precise list, adding nothing new to either):

1. A thin `rusty-clanker-client`-local wrapper type implementing M9-B01's `renderer::Renderer` trait over `rc_render::renderer::TerrainRenderer`, forwarding `GraphicsContext`'s fields into `TerrainRenderer::render`'s plain parameters (M9-B04 §Context 3).
2. `crates/client/src/renderer.rs::GraphicsContext::new`'s `Features::empty()`/`Limits::default()` stub replaced by `rc_render::device::negotiate_device_requirements`'s real output (M9-B04 §Context 3).
3. A production `SnapshotProvider` (M9-B05's trait, unmodified) translating `world::ClientWorld`'s already-real chunk/light data (M9-B03) into `section_snapshot::SectionSnapshot` values, plus the real call sites driving `MeshWorkerPool::mark_dirty`/`mark_dirty_for_block_update` on chunk load/block update and draining completed meshes into `TerrainRenderer::submit_section_mesh` once per frame (M9-B05 §Interfaces).
4. The startup asset-load sequence: `rc_assets::discovery::discover` → `AssetStore::open` → `atlas::discover_block_item_texture_ids` + `AtlasBuilder::build` + `TextureAtlas::upload` → `TerrainRenderer::set_atlas`, and `bake::bake_all` run once at the same point (CLIENT-D14's own "bake once per resource-pack load" cadence).
5. Per-frame: `PlayerController::camera_params_and_update` (M9-B06, already real) fed into `TerrainRenderer::update_camera`, then `TerrainRenderer::render`.

This blueprint's own harness code is written entirely against real B03/B05/B06 types and a real server subprocess, so **no reconciliation of this contract's field names is needed once that blueprint lands** — only this blueprint's own two gated cases (§1.1/§1.2's AC1c/AC2b) flip from `fail` to a real, evidence-backed `pass`, and `docs/MANUAL-VERIFICATION-M9-B07.md` (Deliverables) becomes fully executable rather than partially blocked.

### 3. Leg 1 — real client, real server, block-for-block comparison

**The known pattern, chosen deliberately to need zero server-side custom seeding.** Vanilla's own build-limit floor (`y = -64` at NET-D1's pinned protocol/version) is always `minecraft:bedrock`, unconditionally, for **any** world type, seed, or generation state an M1–M6-feature-complete server produces — a universal, seed-independent, worldgen-type-independent invariant, unlike any specific terrain feature. This is this blueprint's own "known pattern": every block at `y = -64` within the client's initially-loaded chunk area must read back as `minecraft:bedrock`'s real, registry-assigned `BlockStateId` (`rc_registries::generated_v776::block_states`, already a real dependency of `rusty-clanker-client` via B03 — never a hardcoded numeric guess). Choosing an emergent-but-universal invariant over a hand-placed structure sidesteps needing any encode/seed/structure-placement API this blueprint has not independently verified, while still exercising the real thing AC1a actually cares about: does B03's real `LevelChunkWithLight` decode, applied to real bytes a real server sent, land on the exact block-state id the server's own authoritative world holds.

**Real-server harness shape** (`crates/client/tests/common/real_server.rs`, new): spawns `rusty-clanker-server` (the workspace binary, built by `cargo`'s own test-harness dependency resolution — `env!("CARGO_BIN_EXE_rusty-clanker-server")`, the standard Cargo mechanism for a test to launch a sibling workspace binary, no manual path resolution) as a real OS subprocess with `--offline --bind 127.0.0.1:0 --world-dir <fresh tempdir>` (`--bind ... :0` requests an OS-assigned free port; the harness reads the real bound port from the process's own stdout, mirroring `RC_REGION_COUNT`/`RC_REGION_LAYOUT`'s established stdout-contract pattern, M6-B01/M6-B07 — **moderate confidence**: confirm at implementation time that `rusty-clanker-server` prints its bound port on `--bind ...:0`, or use a pre-picked free port via a portable "bind a `TcpListener` to port 0, read back the OS-assigned port, drop it, hand that exact number to `--bind`" probe instead if it does not), waits (bounded retry, short backoff) until the port accepts a TCP connection, and returns a `RealServer` handle whose `Drop` impl sends the process a termination signal and joins it — the identical "own the subprocess's whole lifecycle, terminate on drop" shape `rc-test-harness`'s own TEST-D7 differential harness already establishes for its own vanilla-`server.jar`/Rusty-Clanker-subprocess pair, restated here for a Rusty-Clanker-vs-Rusty-Clanker pair instead.

**The comparison, factored so the mandatory self-test can drive it against a deliberately wrong input:**

```rust
// crates/client/tests/common/real_server.rs

pub struct RealServer { /* child: std::process::Child, port: u16, world_dir: tempfile::TempDir */ }
pub struct RealServerError(pub String); // spawn/port-wait failure, non-panicking

impl RealServer {
    /// Spawns per §Context 3's exact flags; blocks (bounded) until the port is reachable.
    pub fn spawn_offline() -> Result<Self, RealServerError>;
    pub fn addr(&self) -> std::net::SocketAddr;
}
// Drop: best-effort terminate + wait, never panics on an already-dead child.

#[derive(Debug, Clone, PartialEq)]
pub struct BlockMismatch { pub pos: rc_core::BlockPos, pub expected: u32, pub observed: Option<u32> }

/// Pure: checks every `(pos, expected_state_id)` pair against `world`'s own stored state.
/// `observed: None` means the position's chunk was never received at all (a stronger
/// failure than a wrong id — reported identically as a `BlockMismatch`, never a silent skip).
pub fn compare_known_pattern(
    world: &rusty_clanker_client::world::ClientWorld,
    expected: &[(rc_core::BlockPos, u32)],
) -> Vec<BlockMismatch>;

/// The known pattern itself (§Context 3): every `(x, z)` in the sampled set, at `y = -64`,
/// paired with `minecraft:bedrock`'s real default `BlockStateId.0`.
pub fn bedrock_floor_pattern(sample_columns: &[(i32, i32)]) -> Vec<(rc_core::BlockPos, u32)>;
```

`client_world_matches_real_server_bedrock_floor` (Acceptance tests): spawn a `RealServer`; run `rusty_clanker_client::connection::run_client_session` (B03's real entry point) against it with a `LoginIdentity::Offline` identity, driven to completion of its initial chunk-load sequence (the same "wait until `ChunkBatchFinished` observed `N` times" bound B03's own `full_session_walkthrough.rs` already uses); call `bedrock_floor_pattern` for a handful of `(x, z)` columns within the loaded 3×3-or-larger chunk grid B03's own default view distance covers; call `compare_known_pattern` against the resulting `ClientWorld`; assert the returned `Vec` is empty.

**Mandatory self-test**, `divergent_block_fails_the_comparison`: build a small in-memory `ClientWorld` fixture (no real server needed — `compare_known_pattern` is pure), insert one chunk whose `y = -64` block is deliberately **not** bedrock at one sampled position; call `compare_known_pattern` with the identical `bedrock_floor_pattern` expectation; assert the returned `Vec` contains exactly one `BlockMismatch` naming that position — proving the comparison mechanism itself, not merely the happy path, is exercised.

### 4. Leg 1's mesh-render proxy, and Leg 2's position round-trip + frame-time analysis

**Mesh proxy** (`m9_leg1_mesh_render_proxy.rs`): reuses the same `RealServer` + `run_client_session` connection from §Context 3 (a second, independent invocation — this blueprint's own tests never share a live server process across test functions, per `cargo-nextest`'s own per-test process isolation, TEST-D2). After the initial chunk-load sequence, read the received section at the loaded area's origin directly out of `ClientWorld::chunk`/`ClientChunkColumn::get_block_raw` (B03's already-real accessors) and hand-assemble one `rc_render::section_snapshot::SectionSnapshot` from it — **a harness-owned fixture-construction step, not a `SnapshotProvider` implementation**: this blueprint's own test file does the translation once, inline, for exactly this one proof, never claiming to be the production bridge §Context 2 item 3 names. Call `rc_render::bake::bake_all` (against a minimal, in-test-constructed `AssetStore`/blockstate-model fixture set covering `minecraft:bedrock`/`minecraft:stone`/`minecraft:air`, mirroring M9-B05's own fixture-driven testing posture rather than a real `.minecraft` install) to build a `BakedRegistry`, then run one mesh-worker job (or the equivalent direct `mesh.rs` call M9-B05's own headless tests already exercise) against the snapshot. Assert: the resulting `MeshData.opaque.vertices` is non-empty; every vertex's `unpack_pos_and_face_frac` decodes to a position inside the section's own `[0,16)` extent; every vertex's `unpack_material`-resolved texture reference (via the fixture atlas's own `TextureAtlas::resolve`) points at the expected `bedrock`/`stone` texture id for its source block. This proves the render pipeline, fed real network-sourced block data, produces the correct triangles — the honest "automated proxy" this milestone's own task description names, never a substitute for an actual rasterized pixel.

**Position round-trip** (`m9_leg2_position_roundtrip.rs`): a second real `RealServer` + `run_client_session`, this time also constructing a real `player::PlayerController` (B06) and driving its `InputAdapter`/`PredictionSimulation` seams with a fixed, scripted `InputSnapshot` sequence (e.g. forward-held for `M9_AUTOMATED_SESSION_TICKS / 2` ticks, then released) for `M9_AUTOMATED_SESSION_TICKS` ticks (a short, CI-budget-bounded duration — the full continuous 10-minute session is §Context 6's reference-host pass, not this test):

```rust
pub const M9_AUTOMATED_SESSION_TICKS: u32 = 200; // 10 s @ 50 ms — a CI-budget-bounded proxy
    // for AC2a's own "continuous" requirement; the full 10-minute session is a reference-host
    // pass (§Context 6), not re-run here. Seed default, same status every unvalidated numeric
    // threshold in this corpus carries.
```

Assert, across the whole scripted session: every tick's `CadenceState::decide` output (B06, already real) that produces `Some(report)` is observed, byte-exact, on the fake/real server's own receive side (mirroring B06's own `movement_cadence.rs` test 8's fake-server-conformance shape, restated here against a **real** server instead); zero `SynchronizePlayerPosition` correction arrives outside the expected initial spawn-sync one (a correction mid-session would mean client and server prediction diverged — CLIENT-D28's own "a teleport correction should be rare in the ordinary case" made into a concrete, checked assertion); the harness process itself never panics across the whole scripted duration (Rust's own unwind-on-panic plus `cargo-nextest`'s per-test isolation makes this directly, mechanically observable — a panicking test simply fails, per TEST-D2).

**Frame-time / stable-FPS analysis** (`xtask/src/frame_time.rs`, new — pure, no GPU, no window, headlessly testable):

```rust
pub const TARGET_FRAME_BUDGET_MS: f64 = 16.6;      // CLIENT-D32/PERF-D63, 60 FPS
pub const STABLE_P99_CEILING_MS: f64 = 33.3;        // seed default: ~2x budget, a ~30 FPS p99
    // floor — allows the occasional streaming/GC hitch CLIENT-D13 itself frames as expected
    // ("degrades to briefly-reduced edge detail rather than a frame-time spike"), never a
    // sustained miss. Pending real-hardware calibration, same status every other unvalidated
    // numeric threshold in this corpus carries.
pub const MAX_SINGLE_FRAME_STALL_MS: f64 = 250.0;   // a hard "did the process hang" ceiling,
    // independent of the percentile bar above — one very long frame among many good ones would
    // not move p99 much but is still a real stability defect this bar exists to catch.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameTimeReport { pub p50_ms: f64, pub p95_ms: f64, pub p99_ms: f64, pub max_ms: f64, pub stable: bool }

/// Pure: `samples_ms` in emission order (not required sorted). Percentiles via a sorted-copy
/// nearest-rank method (`index = ceil(p * n) - 1`, clamped to `0..n`, a standard, deterministic,
/// dependency-free percentile definition). `stable = p99_ms <= STABLE_P99_CEILING_MS && max_ms
/// <= MAX_SINGLE_FRAME_STALL_MS`. Panics (a caller programming error, not a runtime input) if
/// `samples_ms` is empty.
pub fn analyze_frame_times(samples_ms: &[f64]) -> FrameTimeReport;
```

Fixtures: a "stable" synthetic series (every sample `15.0..=18.0` ms, uniformly, `n = 600` — ten minutes at 60 FPS) asserts `stable == true`. The **mandatory Leg-2 self-test**, `injected_stall_fails_the_stability_check`: the identical stable series with exactly one sample replaced by `400.0` ms (a deliberate stall); assert `stable == false` and `max_ms == 400.0` — proving the analysis function, not merely a hand-picked good series, is what this leg actually gates on.

### 5. Leg 3 — release-artifact content audit, extended

**B02's already-real scanner, restated.** `xtask::content_audit::{Hit, scan, run}` (M9-B02) already walks a directory or file and flags a `.png`/`.ogg`/`.jar` extension or a PNG/OGG/ZIP magic-byte signature. This blueprint's own contribution is **additive**: two new detectors, a best-effort hash cross-check, and — for the first time in this milestone — a real client release-build target and a CI job that actually runs the scan against it.

**New detectors** (`xtask/src/content_audit.rs`, additive — `scan`'s existing signature, behavior, and every one of B02's own `xtask/tests/content_audit.rs` cases are untouched):

```rust
/// Additive to B02's existing `Hit { path, reason }` shape — `reason` gains three new
/// possible values, never removing or renaming the three B02 already established
/// (`"png-extension"`-class strings, restated verbatim from that blueprint, not repeated
/// here — see M9-B02 Deliverables for the exact existing set).
pub const REASON_JPEG_EXTENSION: &str = "jpeg-extension";
pub const REASON_JPEG_MAGIC_BYTES: &str = "jpeg-magic-bytes";
pub const REASON_MODEL_JSON_SIGNATURE: &str = "model-json-signature";
pub const REASON_MOJANG_ASSET_HASH_MATCH: &str = "mojang-asset-hash-match";

#[derive(Default)]
pub struct ScanOptions {
    /// SHA-1 hex digests from a real local `.minecraft` asset index (`rc_assets::asset_index`),
    /// when one is available — `None` in CI, where no such install exists, matching M9-B02's own
    /// "closing note: the non-gating real-jar verification pass" precedent exactly. A scanned
    /// file's own SHA-1 matching an entry here is a hit regardless of extension/magic bytes,
    /// since a renamed/re-extensioned Mojang asset is still a Mojang asset.
    pub known_mojang_hashes: Option<std::collections::HashSet<String>>,
}

/// `scan(root)` is now `scan_with_options(root, &ScanOptions::default())` — B02's own existing
/// callers and tests observe zero behavior change.
pub fn scan(root: &std::path::Path) -> std::io::Result<Vec<Hit>>;
pub fn scan_with_options(root: &std::path::Path, options: &ScanOptions) -> std::io::Result<Vec<Hit>>;

/// Loads `known_mojang_hashes` from a real, legally-obtained local `.minecraft` install if one is
/// discoverable (`rc_assets::discovery::discover(None)` + `asset_index::load_asset_index`); `None`
/// on any discovery/parse failure — never an error, mirroring `rc-assets`' own "a cache is a
/// convenience, never a correctness dependency" stance (M9-B01 §Context 7) applied here to an
/// audit enhancement rather than a config file.
pub fn try_load_known_mojang_hashes() -> Option<std::collections::HashSet<String>>;
```

Detection rules, each additive to B02's existing per-file check: JPEG extension (`.jpg`/`.jpeg`, case-insensitive) or magic bytes (`FF D8 FF`, the first 3 bytes) → `REASON_JPEG_EXTENSION`/`REASON_JPEG_MAGIC_BYTES`. Model-JSON signature: attempt `rc_assets::blockstate::parse_blockstate`/`rc_assets::model::parse_model` (already-real, already-audited B02 parsers — reused, never a second hand-rolled JSON heuristic) against the file's bytes; a successful `parse_blockstate` with `variants.is_some() || multipart.is_some()`, or a successful `parse_model` with `elements.is_some() || parent.is_some()`, is a hit (`REASON_MODEL_JSON_SIGNATURE`) — a real, not merely heuristic, structural match, since both are the exact production parsers a real blockstate/model file must satisfy. Hash cross-check: when `known_mojang_hashes` is `Some`, every scanned file's SHA-1 (`sha1`, already pinned) is checked for membership, independent of the other three checks.

**The allowlist — what may legitimately be embedded, restated from `08-assets-auth-legal.md`'s custody rules (ASSET-D13/D16/D24), applied to M9's own concrete artifact:** own-authored WGSL shader source (M9-B04's `shaders/terrain_{bindless,tiered}.wgsl`, plain text embedded via `include_str!`, never a binary asset signature, never Mojang-derived — code, not content); the compiled binary itself and ordinary build metadata (`Cargo.lock`-derived version strings); own-authored `LICENSE`/`README`/the ASSET-D22 disclaimer text (plain text). **Nothing else is legitimately embeddable at M9**: no font (CLIENT-D17 is M10 scope), no icon (M9-B01's own `WindowAttributes::default()` sets none), no fixture/test data (fixtures live under `tests/`, never shipped in a release archive, and are excluded from the scan by scanning only the packaged release tree, never the source checkout). The scanner's own **superset** design (extension-or-magic-bytes, per M9-B02's own stated rationale) is therefore correct with **zero allowlist exceptions configured** — any hit anywhere in the packaged tree is real and must fail the build.

**Real client release build + scan** (`xtask/src/client_release_audit.rs`, new): `cargo run -p xtask -- client-release-audit --out-dir <dir>` runs `cargo build --release -p rusty-clanker-client` under M6-B05 §D's already-pinned `-C target-cpu=x86-64-v2` policy (restated, not re-decided — no PGO/BOLT is applied here, since M6-B05 §E.1's own workload is explicitly server-only and M9's own milestone scope names no client-PGO requirement; this is a plain, maximally-LTO'd `[profile.release]` build, §Context per M6-B05 §B, unmodified), copies the resulting binary plus `LICENSE`/`README.md` into `<out-dir>/rusty-clanker-client-release/`, runs `scan_with_options` against that directory with `try_load_known_mojang_hashes()`'s result, and writes one `CaseResult` (§Context 7) reporting every `Hit` found, non-zero exit on any.

### 6. Reference-host schema extension — a fourth, GPU-bearing tier

**Additive, backward-compatible.** M6-B04's `ReferenceHostSpec`/`ReferenceHostTier`/`HostFingerprint`/`match_tier` are extended, never rewritten: `SPEC_SCHEMA_VERSION` becomes `2`; `TierId` gains one new variant, `M9ClientReference`; `KNOWN_TIER_IDS` becomes a 4-entry array; `validate_spec`'s check (2) becomes `tier.len() == 4`. `ReferenceHostTier` gains one new, `#[serde(default)]`-optional field, `gpu: Option<GpuRequirement>` — every existing tier's TOML entry (`dev-workstation`/`m6-acceptance`/`budget-vps`) omits it, exactly as they already omit `cpu_governor`/`max_timer_granularity_micros` where not gated, so **their own committed values and match behavior are byte-for-byte unchanged.** `HostFingerprint` gains one new, always-optional field, `gpu: Option<GpuFingerprint>` — `probe_host()`'s own existing, real-GPU-free implementation is untouched and simply leaves it `None` (this blueprint adds no `wgpu` dependency to `xtask`, keeping the probe's own headless-testability guarantee, M6-B04 §"Host probing," intact).

```rust
// additive to xtask/src/reference_host.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TierId { DevWorkstation, M6Acceptance, BudgetVps, M9ClientReference }
pub const KNOWN_TIER_IDS: [TierId; 4] =
    [TierId::DevWorkstation, TierId::M6Acceptance, TierId::BudgetVps, TierId::M9ClientReference];

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GpuRequirement {
    /// Documentation only, per M6-B04's own established precedent for `cpu_model_class`
    /// (never itself matched by `match_tier` — only `logical_cores`/`ram_gib`/etc. are).
    pub model_class: String,
    /// The one GPU field `match_tier` actually checks — a floor, no tolerance band needed
    /// (unlike `ram_gib`'s 10%, a VRAM floor is unambiguous: more is always fine).
    pub vram_gib: u32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GpuFingerprint { pub name: Option<String>, pub vram_gib: Option<u32> }

// ReferenceHostTier gains: #[serde(default)] pub gpu: Option<GpuRequirement>,
// HostFingerprint gains: #[serde(default)] pub gpu: Option<GpuFingerprint>,

/// Pure — no `wgpu` call anywhere in `xtask` (Context, this section). Constructs a
/// `GpuFingerprint` from operator-supplied, already-obtained values (§Deliverables' CLI
/// `--gpu-info <path>` reads a small `{ "name": "...", "vram_gib": 12 }` JSON file the manual
/// procedure instructs the operator to fill in from their OS's own GPU info panel — the same
/// "the one thing automation on this host cannot close, named and documented, never silently
/// assumed" category M9-B01/M9-B04's own manual-verification docs already use for real
/// window/GPU bootstrap).
pub fn gpu_fingerprint_from_json(bytes: &[u8]) -> Result<GpuFingerprint, serde_json::Error>;

/// `match_tier`'s new, 12th check, appended after the existing 11 (order: `field = "gpu"`,
/// gate: `declared.gpu.is_none()`, match: `fingerprint.gpu.as_ref().and_then(|g| g.vram_gib)`
/// is `Some(x)` with `x >= declared.gpu.as_ref().unwrap().vram_gib`).
```

`reference-hosts.toml` gains one new `[[tier]]` block, `id = "m9-client-reference"`, `authoritative = true`, matching PERF-D64's own already-committed profile verbatim: `cpu_model_class = "8 cores at a ~4.5 GHz single-thread boost class"`, `logical_cores = 16`, `physical_cores = 8`, `smt = "on"`, `ram_gib = 16`, `storage_class = "nvme-ssd"`, `gpu.model_class = "discrete GPU, mid-2020s-mid-range compute tier or later (e.g. RTX 4060 / RX 7600-class)"`, `gpu.vram_gib = 8`, `os_id`/`os_version_id`/`kernel_line` **omitted** (this tier is deliberately OS-agnostic — TEST-D34's client CI matrix spans both `ubuntu-24.04` and `windows-2025`, and PERF-D64 itself pins no OS; making these three fields checkable-when-declared would require widening them to `Option<String>` for every tier, a larger schema change than this blueprint's own scope needs — flagged as the concrete reason those fields stay `String`/required on the three pre-existing tiers and are simply never declared, hence never gated, on this fourth one, per `#[serde(default)]`'s "omitted key" contract already established for every other optional field in this schema), `cgroup_quota_enforced = false`, `source_decision_ids = ["PERF-D64", "M9-B07"]`.

`docs/CONTRIBUTING.md`'s protected-path table needs no new row — `reference-hosts.toml` is already protected (M6-B04's own row); this blueprint's changeset touching it is therefore governed by the identical `Changeset-Type: governance` rule M6-B04 itself already applied to its own first edit of this same file (Constraints).

### 7. The M9 completion report and manual-evidence gating

```rust
// xtask/src/m9_report.rs — mirrors M8-B05's own m8_report.rs shape exactly.

pub const OUT_PATH: &str = "target/verify/m9-acceptance.json";

#[derive(serde::Deserialize)]
pub struct ManualEvidence {
    pub tier: crate::reference_host::TierId,          // must be M9ClientReference
    pub fingerprint: crate::reference_host::HostFingerprint,
    /// SHA-256 of a captured screenshot file — recorded as attached diagnostic evidence for a
    /// human reviewer, per 09's own "a hash mismatch triggers a full-fidelity dump-and-diff...
    /// never just a pass/fail bit" framing (TEST-D10) applied here to visual verification: this
    /// field is never itself compared against anything and never gates `pass`/`fail` on its own
    /// — its only role is proving a screenshot was actually captured during the same session
    /// the other fields describe, not the correctness of its pixels.
    pub screenshot_sha256: Option<String>,
    pub frame_time_samples_ms: Option<Vec<f64>>,
    pub session_duration_ticks: Option<u64>,
    pub zero_crashes: bool,
    pub auth_account_username: Option<String>,         // never an access token (ASSET-D10)
    pub commit_hash: String,
    pub tested_at: String,
}

/// Pure: `true` iff `evidence.zero_crashes`, `evidence.session_duration_ticks` is `Some(d)` with
/// `d >= M9_MANUAL_SESSION_MIN_TICKS` (10 real minutes at 20 TPS = 12_000), and
/// `evidence.frame_time_samples_ms` is `Some(s)` with `frame_time::analyze_frame_times(s).stable`.
pub fn manual_session_meets_bar(evidence: &ManualEvidence) -> bool;
pub const M9_MANUAL_SESSION_MIN_TICKS: u64 = 12_000;

/// Reads and parses a `ManualEvidence` JSON file, wraps it via
/// `reference_host::gate(evidence, evidence.fingerprint.clone(), declared_tier)` (M6-B04's own
/// mechanism, reused unmodified) and folds `authoritative && manual_session_meets_bar(&evidence)`
/// into Leg 1c's/Leg 2b's own `CaseResult`s. `None` input (no `--manual-evidence` given) reports
/// both `fail` with an actionable "run docs/MANUAL-VERIFICATION-M9-B07.md and supply
/// --manual-evidence" message, exactly M8-B05's own `junit_path`-absent framing.
pub fn evidence_derived_cases(
    evidence: Option<&ManualEvidence>,
    spec: &crate::reference_host::ReferenceHostSpec,
) -> Vec<crate::tier_result::CaseResult>;

pub struct M9ReportArgs {
    pub out_dir: std::path::PathBuf,
    pub manual_evidence: Option<std::path::PathBuf>,
}

/// Runs Legs 1a/1b/2a/3 for real via `cargo nextest`-produced JUnit XML if a `--junit-path` is
/// additionally supplied (mirroring M8-B05's own sourcing pattern exactly — implementer's own
/// choice between re-running nextest internally or requiring a pre-produced JUnit file, same
/// tradeoff M8-B05 already resolved in favor of the latter), folds in `evidence_derived_cases`,
/// writes `OUT_PATH`, returns the matching exit code.
pub fn run(args: &M9ReportArgs, junit_path: Option<&std::path::Path>) -> std::process::ExitCode;

/// Pure aggregation, independently tested against synthetic inputs (mirroring M6-B06/M7-B09/
/// M8-B05's own `build_report`).
pub fn build_report(
    junit_cases: Vec<crate::tier_result::CaseResult>,
    evidence_cases: Vec<crate::tier_result::CaseResult>,
    content_audit_case: crate::tier_result::CaseResult,
) -> crate::tier_result::TierResult; // tier = "m9-acceptance"
```

Case names, `tier: "m9-acceptance"`: `leg1a_block_placement_matches_server` (JUnit-sourced, from `m9_leg1_block_placement`), `leg1b_mesh_render_proxy` (JUnit-sourced, from `m9_leg1_mesh_render_proxy`), `leg1c_auth_and_screenshot` (evidence-derived — `fail` with the §Context 2-citing message absent evidence), `leg2a_position_roundtrip` (JUnit-sourced, from `m9_leg2_position_roundtrip`), `leg2b_reference_gpu_frame_rate_session` (evidence-derived, same treatment as `leg1c`), `leg3_release_artifact_content_audit` (real, run internally by `client_release_audit::run`, never JUnit-sourced — it is a full `cargo build --release` plus a scan, not a nextest case).

## Deliverables

### `crates/client/tests/common/real_server.rs` (new)

Per Context §3, full signatures given verbatim above: `RealServer`, `RealServerError`, `RealServer::{spawn_offline, addr}`, `BlockMismatch`, `compare_known_pattern`, `bedrock_floor_pattern`.

### `crates/client/tests/m9_leg1_block_placement.rs` (new)

Per Acceptance tests, below.

### `crates/client/tests/m9_leg1_mesh_render_proxy.rs` (new)

Per Context §4, first half.

### `crates/client/tests/m9_leg2_position_roundtrip.rs` (new)

Per Context §4, second half.

### `xtask/src/content_audit.rs` (modify — additive; every existing signature/behavior from M9-B02 unchanged)

`REASON_JPEG_EXTENSION`, `REASON_JPEG_MAGIC_BYTES`, `REASON_MODEL_JSON_SIGNATURE`, `REASON_MOJANG_ASSET_HASH_MATCH`, `ScanOptions`, `scan_with_options`, `try_load_known_mojang_hashes` — full signatures per Context §5.

### `xtask/src/client_release_audit.rs` (new)

```rust
pub struct ClientReleaseAuditArgs { pub out_dir: std::path::PathBuf }

#[derive(Debug, thiserror::Error)]
pub enum ClientReleaseAuditError {
    #[error("cargo build --release -p rusty-clanker-client failed: {0}")]
    BuildFailed(String),
}

/// Builds (§Context 5), packages, scans, writes one `CaseResult` naming every `Hit`.
pub fn run_audit(args: &ClientReleaseAuditArgs)
    -> Result<crate::tier_result::CaseResult, ClientReleaseAuditError>;
/// CLI entry point: `run_audit` + `tier_result::write` under `tier: "client-release-audit"` +
/// exit code.
pub fn run(args: &ClientReleaseAuditArgs) -> std::process::ExitCode;
```

### `xtask/src/frame_time.rs` (new)

Per Context §4, full signatures given verbatim above: `TARGET_FRAME_BUDGET_MS`, `STABLE_P99_CEILING_MS`, `MAX_SINGLE_FRAME_STALL_MS`, `FrameTimeReport`, `analyze_frame_times`.

### `xtask/src/reference_host.rs` (modify — additive per Context §6)

`TierId::M9ClientReference` variant, `KNOWN_TIER_IDS` widened to 4, `validate_spec`'s tier-count check updated to `4`, `GpuRequirement`, `GpuFingerprint`, `ReferenceHostTier.gpu`/`HostFingerprint.gpu` new optional fields, `gpu_fingerprint_from_json`, `match_tier`'s 12th check. `SPEC_SCHEMA_VERSION` becomes `2`.

### `reference-hosts.toml` (modify — additive per Context §6)

`schema_version = 2`; one new `[[tier]]` block, `id = "m9-client-reference"`, full field set per Context §6's exact values.

### `xtask/src/m9_report.rs` (new)

Per Context §7, full signatures given verbatim above.

### `xtask/src/lib.rs` (modify — three new `pub mod` lines, additive)

```rust
pub mod client_release_audit;
pub mod frame_time;
pub mod m9_report;
```

### `xtask/src/main.rs` (modify — three new `Command` variants, additive; existing variants/arms unchanged)

```rust
/// M9-B07: real client release build + extended content-audit scan.
ClientReleaseAudit { #[arg(long)] out_dir: std::path::PathBuf },
/// M9-B07: the M9 completion report.
M9Report {
    #[arg(long)] out_dir: std::path::PathBuf,
    #[arg(long)] manual_evidence: Option<std::path::PathBuf>,
    #[arg(long)] junit_path: Option<std::path::PathBuf>,
},
```

`HostFingerprint`'s existing `Command::HostFingerprint { tier, spec }` (M6-B04) gains one additive field, `#[arg(long)] gpu_info: Option<String>` — when present, `reference_host::run` (M6-B04, modified additively) reads and merges a `GpuFingerprint` via `gpu_fingerprint_from_json` into the probed fingerprint before matching; absent, `fingerprint.gpu` stays `None` exactly as it already does for every non-`m9-client-reference` tier.

### `.github/workflows/ci.yml` (modify — one new, additive `workflow_dispatch`-only job)

`client-release-audit`, mirroring `reference-host-gate`'s own shape (M6-B04): `on: workflow_dispatch`, `runs-on: [ubuntu-24.04, windows-2025]` matrix (unlike `reference-host-gate`, this job needs no self-hosted reference hardware — a plain release build + byte scan runs correctly on any GitHub-hosted runner, since it proves an artifact-content property, not a performance one), one step: `cargo run -p xtask -- client-release-audit --out-dir target/verify`. Not part of the required Tier-1 status-check set (Done-bar).

### `docs/MANUAL-VERIFICATION-M9-B07.md` (implementer creates; content this blueprint specifies)

A consolidated, reproducible reference-host procedure, extending — never duplicating — B03 §15's own already-committed step list: **(1)** run B03 §15's own auth-then-connect procedure verbatim (device-code sign-in, real `AuthSession`, `keyring` entry confirmed, `try_resume` silent-refresh confirmed) against a real, then-integrated (§Context 2) `rusty-clanker-client` binary connecting to a real `online_mode = true` M1–M6-feature-complete server; **(2)** once connected and rendering, capture one screenshot of the rendered terrain and record its SHA-256; **(3)** drive a continuous 10-minute (`M9_MANUAL_SESSION_MIN_TICKS`, 12,000 ticks) camera-movement session (WASD + mouse-look), recording per-frame timestamps into a CSV; **(4)** fill in `{ "name": "<GPU model>", "vram_gib": <N> }` from the OS's own GPU info panel and run `cargo run -p xtask -- host-fingerprint --tier m9-client-reference --gpu-info <that file> --out-dir <dir>`; **(5)** assemble a `ManualEvidence` JSON (Context §7's schema) from steps 1–4's outputs plus the fingerprint from step 4 and the current commit hash; **(6)** run `cargo run -p xtask -- m9-report --out-dir <dir> --manual-evidence <that file> --junit-path <a prior `cargo nextest run -p rusty-clanker-client -p xtask` JUnit output>` and confirm every case, including `leg1c_auth_and_screenshot`/`leg2b_reference_gpu_frame_rate_session`, reports `pass`. Steps 1–3 are honestly blocked until §Context 2's integration blueprint lands — this document states that plainly rather than presenting a procedure that cannot yet be executed as if it already could be.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, binding):** `crates/client/tests/{common/real_server, m9_leg1_block_placement, m9_leg1_mesh_render_proxy, m9_leg2_position_roundtrip}.rs` and `xtask/tests/{content_audit_release, frame_time, m9_report, reference_host_gpu_tier}.rs`, plus every new/modified `xtask/src/*.rs` function body from Deliverables `todo!()`-stubbed (types/enums fully defined), are committed first. The implementation changeset fills in real bodies only; it must not modify any file under `crates/client/tests/`, `xtask/tests/`, or any file under a **prior** blueprint's own `tests/` directory (M9-B02's `xtask/tests/content_audit.rs`; M9-B03's `crates/client/tests/*.rs`; M9-B05's/M9-B06's own suites), and must not weaken, delete, or `#[ignore]` any named case below.

- `common/real_server.rs`: not itself a test file — a shared helper, `mod common;`-included by the two `m9_leg*` files that need it (mirroring M9-B02/M8-B05's own established `tests/common/` convention).
- `m9_leg1_block_placement.rs`: `client_world_matches_real_server_bedrock_floor` (Context §3, full real-server scenario). `divergent_block_fails_the_comparison` **(mandatory self-test)** (Context §3, pure, no server). `bedrock_floor_pattern_uses_the_real_registry_id` — `bedrock_floor_pattern(&[(0,0)])[0].1` equals `rc_registries::generated_v776::block_states`'s own real, looked-up default state id for `minecraft:bedrock` (never a hardcoded literal — proves this test suite's own oracle is the real registry, not a guess).
- `m9_leg1_mesh_render_proxy.rs`: `real_chunk_data_produces_correct_baked_mesh` (Context §4, full scenario). `mesh_is_empty_when_section_is_all_air` — a hand-built all-air `SectionSnapshot` (no server needed) produces `MeshData` with every `LayerMesh` empty, a cheap sanity precondition on the same bridge code the real-server case depends on.
- `m9_leg2_position_roundtrip.rs`: `scripted_movement_round_trips_with_zero_desync` (Context §4, full scenario). `no_unexpected_mid_session_sync_packet` — same scenario, asserting the mid-session `SynchronizePlayerPosition` count observed is exactly `0` beyond the initial spawn sync.
- `xtask/tests/content_audit_release.rs` (new — never touches B02's own `content_audit.rs` test file): `flags_jpeg_extension`, `flags_jpeg_magic_bytes_regardless_of_extension`, `flags_a_valid_blockstate_json_by_signature`, `flags_a_valid_model_json_by_signature`, `does_not_flag_ordinary_rust_or_wgsl_source`, `hash_crosscheck_flags_a_known_hash_regardless_of_extension` (a `.txt` file whose bytes hash to an entry in a synthetic `known_mojang_hashes` set), `hash_crosscheck_is_skipped_gracefully_when_no_hashes_supplied` (`ScanOptions::default()`, no panic, no false hit). `embedded_png_in_a_release_archive_tree_fails_the_scan` **(mandatory self-test)** — a synthetic directory tree shaped like `client_release_audit`'s own packaged output, with one deliberately embedded PNG fixture; `scan_with_options` returns a non-empty `Vec`.
- `xtask/tests/frame_time.rs`: `stable_series_reports_stable` (Context §4's fixture). `injected_stall_fails_the_stability_check` **(mandatory self-test)** (Context §4's fixture). `percentiles_are_computed_via_nearest_rank` — a small, hand-computed 10-sample series with known p50/p95/p99 values, exact-match asserted.
- `xtask/tests/reference_host_gpu_tier.rs`: `committed_spec_still_validates_with_four_tiers` — `load_spec`/`validate_spec` against the real, modified `reference-hosts.toml` succeeds. `gpu_gate_skipped_when_not_declared` — `match_tier` against `m6-acceptance` (no `gpu` field) never produces a `"gpu"` `FieldCheck` outside `NotGated`. `gpu_gate_matches_on_vram_floor` — a fingerprint with `vram_gib: Some(12)` against a declared `vram_gib: 8` → `Matched`; `vram_gib: Some(4)` → `Mismatch`. `gpu_fingerprint_from_json_round_trips` — a small fixture JSON parses to the expected `GpuFingerprint`.
- `xtask/tests/m9_report.rs`: `no_evidence_reports_leg1c_and_leg2b_as_fail_with_actionable_message`. `evidence_with_matching_tier_and_stable_session_reports_pass` (a hand-built `ManualEvidence` fixture whose `fingerprint` exactly matches the committed `m9-client-reference` tier and whose `frame_time_samples_ms` is the "stable" fixture from `frame_time.rs`). `evidence_with_mismatched_gpu_reports_fail` (fixture's `fingerprint.gpu.vram_gib` below the declared floor). `evidence_with_unstable_session_reports_fail` (fixture's `frame_time_samples_ms` is the stall-injected series). `build_report_aggregates_all_six_cases`.

## Implementation steps

1. **`xtask/src/frame_time.rs`.** Implement `analyze_frame_times` per Context §4's exact percentile/stability rule. Observable: `frame_time.rs`'s 3 cases pass.
2. **`xtask/src/content_audit.rs` delta.** Implement `ScanOptions`/`scan_with_options` (extending `scan`'s existing internal walk with the JPEG/model-JSON/hash checks, additive), `try_load_known_mojang_hashes`; rewrite `scan` as a one-line forward to `scan_with_options(root, &ScanOptions::default())`. Observable: `content_audit_release.rs`'s cases pass; B02's own `xtask/tests/content_audit.rs` still passes unmodified.
3. **`xtask/src/reference_host.rs` delta.** Add `TierId::M9ClientReference`, widen `KNOWN_TIER_IDS`/`validate_spec`'s count check, `GpuRequirement`/`GpuFingerprint`/the two new optional fields, `gpu_fingerprint_from_json`, `match_tier`'s 12th check, bump `SPEC_SCHEMA_VERSION`. Observable: `reference_host_gpu_tier.rs`'s cases pass.
4. **`reference-hosts.toml` delta.** Add the `m9-client-reference` block per Context §6's exact values, bump `schema_version`. Observable: `committed_spec_still_validates_with_four_tiers` passes.
5. **`crates/client/tests/common/real_server.rs`.** Implement `RealServer::spawn_offline`/`addr`/`Drop`, `compare_known_pattern`, `bedrock_floor_pattern` per Context §3. Observable: compiles; `bedrock_floor_pattern_uses_the_real_registry_id` and `divergent_block_fails_the_comparison` pass (neither needs a real server).
6. **`m9_leg1_block_placement.rs`.** Wire `client_world_matches_real_server_bedrock_floor` per Context §3. Observable: passes against a real, locally-built `rusty-clanker-server`.
7. **`m9_leg1_mesh_render_proxy.rs`.** Wire the fixture bridge and both cases per Context §4. Observable: both pass.
8. **`m9_leg2_position_roundtrip.rs`.** Wire the scripted-session scenario and both cases per Context §4. Observable: both pass.
9. **`xtask/src/client_release_audit.rs`.** Implement `run_audit`/`run` per Context §5/Deliverables. Observable: `cargo run -p xtask -- client-release-audit --out-dir target/verify` succeeds against this repository's own current source with zero hits.
10. **`xtask/src/m9_report.rs`.** Implement `manual_session_meets_bar`, `evidence_derived_cases`, `run`, `build_report` per Context §7. Observable: `m9_report.rs`'s 5 cases pass.
11. **`xtask/src/lib.rs`, `xtask/src/main.rs`.** Add the three module declarations and three `Command` variants (plus `HostFingerprint`'s additive `gpu_info` field and its dispatch-site wiring). Observable: `cargo run -p xtask -- m9-report --help`/`client-release-audit --help` both succeed.
12. **`.github/workflows/ci.yml`.** Add the `client-release-audit` job per Deliverables. Observable: the workflow file parses (`actionlint`, if configured, or a manual review — no automated CI-syntax test exists in this corpus's own xtask suite, matching M6-B04's own identical treatment of `reference-host-gate`'s addition).
13. **`docs/MANUAL-VERIFICATION-M9-B07.md`.** Write per Deliverables' content list.
14. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test`, `-- path-guard` — all five exit 0.
15. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding (TEST-D45/D46).** Every file named in Acceptance tests is committed first, against `todo!()`-stubbed bodies with the Deliverables' exact signatures. The implementation changeset fills in real bodies only — it must not edit any file under `crates/client/tests/` or `xtask/tests/`, must not weaken any assertion, and must not touch any file under a prior blueprint's own `tests/` directory.

(b) **Changeset labeling (M0-B08/M6-B04's own established convention, restated and binding).** This blueprint's changes fall into two groups, committed separately: `crates/client/tests/**` is an ordinary `test-authoring`/`implementation` pair (TEST-D45). Every change under `xtask/**`, `reference-hosts.toml`, and `.github/workflows/ci.yml` — including files this blueprint only *adds* — is committed as one changeset labeled `Changeset-Type: governance`, never `implementation`, mirroring M6-B04 §"TEST-D46" and M6-B05/M7-B09's own identical restatement of the rule for exactly this class of change (`xtask/**` is already row 7 of the TEST-D46 protected-path table from M0-B08 onward; `reference-hosts.toml` is already protected by M6-B04's own row — no new `PROTECTED_PATHS` row is needed by this blueprint).

(c) **No new external dependencies beyond the pinned set.** `tempfile` (already a dev-dependency precedent elsewhere in this corpus — verify it is already `[workspace.dependencies]`-pinned at implementation time; if not, this is the one blueprint-named addition this task requires, mirroring M9-B01's `tracing-subscriber`/M9-B04's `glam`/`bytemuck` "name it in the blueprint" precedent), `sha1`, `serde_json`, `thiserror` are already pinned. No `wgpu` dependency is added to `xtask` (Context §6 — the GPU fingerprint stays a plain, operator-supplied data structure). No new dependency is added to `rusty-clanker-client` at all — this blueprint's client-side content is entirely `tests/`-scoped.

(d) **No Mojang or third-party reimplementation code.** The bedrock-floor invariant (Context §3) is public, universally-known vanilla behavior (every build ends in a bedrock floor at the pinned protocol's world-height floor) restated as a test oracle, not sourced from any decompiled or leaked reference. Every algorithm this blueprint's Deliverables use (percentile computation, the free-list-adjacent subprocess-lifecycle pattern, the extension/magic-byte/signature scanner) is general, independently-documented technique or this blueprint's own composition of already-real, already-audited B02/B03/B05/B06 code. ASSET-D18/D19/D30 apply and are inherited, not newly load-bearing here.

(e) **The composition-root gap (Context §2) is a named, deliberate deferral — never quietly filled.** This blueprint does not implement a `Renderer`-trait wrapper, does not replace `GraphicsContext::new`'s feature-negotiation stub, does not implement a production `SnapshotProvider`, and does not construct a real `AssetStore`/`TextureAtlas`/`TerrainRenderer` wired into `rusty-clanker-client`'s own `main.rs`/`app.rs`. Adding any placeholder implementation of these "to make Legs 1c/2b look automated" would misrepresent a real, honestly-disclosed gap as closed — `leg1c_auth_and_screenshot`/`leg2b_reference_gpu_frame_rate_session` reporting `fail` absent `--manual-evidence` is this blueprint's own correct, expected behavior, not a defect to work around.

(f) **No scope creep beyond this blueprint's own three-leg wiring.** No new mechanics, protocol packet, or rendering feature is added anywhere. `rc_render::bake::bake_all`/mesh-job invocation in `m9_leg1_mesh_render_proxy.rs` calls M9-B05's own already-real functions unmodified — this blueprint never edits `crates/render/src/{bake,mesh,section_snapshot}.rs`.

(g) **No `unsafe` code.** Every deliverable in this blueprint is ordinary safe Rust; subprocess spawning/termination uses only `std::process`'s safe API.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rusty-clanker-client -p xtask --all-features
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- path-guard
cargo nextest run -p rusty-clanker-client -p xtask
cargo test --doc -p rusty-clanker-client -p xtask
cargo run -p xtask -- client-release-audit --out-dir target/verify
cargo run -p xtask -- m9-report --out-dir target/verify --junit-path target/nextest/default/junit.xml
```

Expected: every command exits 0; the final command's `target/verify/m9-acceptance.json` reports `leg1a`/`leg1b`/`leg2a`/`leg3` as `pass` and `leg1c`/`leg2b` as `fail`, each with the exact, actionable, §Context 2-citing message — this is this blueprint's own correct, expected Done state until a future client-integration blueprint satisfies that contract, not a defect. CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs (Tier 1) is the authoritative done-signal (TEST-D50) for everything except `client-release-audit`, which is `workflow_dispatch`-only and verified by a maintainer running it manually once after merge.

## Interfaces

**Needs from a future client-integration blueprint (Context §2's binding contract, restated as the single most load-bearing open item this blueprint hands forward):** the `Renderer`-trait wrapper, real device-feature negotiation, the production `SnapshotProvider`, and the startup asset-load sequence — once that blueprint lands, this blueprint's own `docs/MANUAL-VERIFICATION-M9-B07.md` steps 1–3 become executable for the first time, and `leg1c_auth_and_screenshot`/`leg2b_reference_gpu_frame_rate_session` flip from a documented `fail` to a real, evidence-backed `pass` with no code change to this blueprint's own Deliverables — only a `--manual-evidence` file needs to be supplied.

**Provides to that future blueprint:** `player::PlayerController::camera_params_and_update` and `rc_render::renderer::TerrainRenderer`'s own already-real API (M9-B04/M9-B06, unchanged by this blueprint) remain exactly what that blueprint wires together; this blueprint adds no new constraint on either.

**Provides to `09-testing-quality.md`:** a second, concrete instance (alongside M8-B05's own) of the "pin the contract, prove everything else hermetically, fail closed" harness-blueprint pattern, now applied to a genuinely different kind of gap (a rendering/composition-root integration, not a mod-loading one) — confirming the pattern generalizes, not merely a one-off for M8's own specific gap.

**Provides to a future `12-workspace-structure.md` revision:** `reference-hosts.toml`'s schema-version bump and fourth tier (Context §6) should be folded into that document's own eventual restatement of the reference-host mechanism, mirroring every other "should be folded back into X on that document's next revision" note this corpus already carries for blueprint-derived reconciliations.

## Open Questions

- `M9-B00-index.md` does not yet exist for this milestone (every other milestone, `M0`–`M8`, has one) — out of this blueprint's own assigned scope to create, flagged here so a maintainer notices the gap rather than assuming it was overlooked silently.
- Whether `rusty-clanker-server` actually prints its bound port to stdout when given `--bind ...:0` (Context §3's `RealServer::spawn_offline`) is not independently verified against M6-B07's own committed `main.rs` content — the fallback (probe a free port before spawning, pass it explicitly) is named as the concrete alternative if the stdout contract does not exist; either way, `RealServer`'s own public surface (`spawn_offline`/`addr`) is unaffected.
- `STABLE_P99_CEILING_MS`/`MAX_SINGLE_FRAME_STALL_MS`/`M9_AUTOMATED_SESSION_TICKS` are seed defaults pending real reference-hardware calibration, the identical status every other unvalidated numeric threshold in this corpus carries.
- Whether `client-release-audit`'s plain (non-PGO) release build should eventually be folded into M6-B05's own `xtask release` verb as a third `--target-crate client` option, rather than staying a separate, narrower verb — left to a future revision's judgment; this blueprint's own narrower verb is sufficient for ASSET-D24's own content-audit requirement, which needs a real build, not a PGO-optimized one.
- Whether `tempfile` is already workspace-pinned (Constraints (c)) needs a direct check of the current root `Cargo.toml` at implementation time — if absent, this blueprint's own named addition, per the established "name it in the blueprint" exception.
