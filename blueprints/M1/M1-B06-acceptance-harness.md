# M1-B06 — Acceptance Harness: Status Probe, Bot-Driver Idle-Stability Leg, CI Tiers, M1-Completion Report

| Field | Content |
|---|---|
| ID | M1-B06 |
| Milestone | M1 — Protocol Bootstrap: Status & Login |
| Prerequisites | M1-B01 (framing/codec — `rc-protocol`'s `VarInt`/frame/wire layer this blueprint's fake server reuses directly). M1-B02 (Handshake `Intention` and the Status/Ping flow — its exact, authoritative wire facts are folded into this blueprint's own Fake-Server Protocol Cheat Sheet in place of independent research, and its own `status_probe_returns_expected_json_and_ping_pong` in-process test is the unit-level counterpart this blueprint's own external, real-subprocess probe complements — see Context, "Relationship to M1-B02's own status test"). M1-B04 ("Login, Configuration, and the Handoff into Play" — the Login/Configuration packet catalogs and the state-machine driver that gets a connection to `ConnectionState::Play`). M1-B05 ("Minimal Play State: Superflat Placeholder, Spawn, Keep-Alive" — the Play-state placeholder-world spawn *and* keep-alive/timeout/clean-disconnect behavior, both in this one blueprint, not two). M1-B02/M1-B04/M1-B05 are none of them named as a Cargo/Rust-API dependency of this blueprint, only as the real, already-merged blueprints whose *combined external wire behavior* this blueprint measures: this blueprint treats `rusty-clanker-server` as an **opaque network peer**, exactly as `09-testing-quality.md`'s TEST-D7 differential-harness architecture already treats every server under test — every one of this blueprint's own deliverables talks to a server binary only over a real TCP socket, never via a Rust API/Cargo dependency on `rusty-clanker-server`'s internals. The one thing this blueprint *does* assume from those prerequisites is a minimal external contract — an overridable bind address and an offline-mode toggle on the server binary's CLI, plus a real `main.rs`/composition root that M1-B02 itself explicitly states it does **not** build — restated precisely in Context, "Assumed server CLI surface," which is either already true by the time this blueprint is implemented or is this blueprint's own small, explicitly-scoped addition to `rusty-clanker-server`'s `main.rs` if not. Also depends on M0-B01 (workspace scaffold, `xtask`'s `Command` enum and verb-dispatch pattern) and M0-B08 (verification wiring — this blueprint reuses `xtask::tier_result::{TierResult, CaseResult, Status}` and `xtask::path_guard`'s `PROTECTED_PATHS`/glob-match machinery unmodified in shape, extending only their *data*, never their logic). |
| Implements | TEST-D7 (differential-harness architecture — restated and deliberately **narrowed** for M1: one server under test, no second vanilla process to diff against yet, since M1's placeholder world has no real content to compare — see Context, "Scope: not yet TEST-D7's full two-server diff"); TEST-D8 (azalea as the bot driver — concretely wired here for the first time); TEST-D37/TEST-D40 (this blueprint's own CI-tier placement and machine-readable JSON output, restated concretely below); TEST-D38/TEST-D41/TEST-D48 (the oracle/vanilla-jar boundary — restated as **not needed by this blueprint's own automated tests**, see Context); TEST-D46 (extends `PROTECTED_PATHS` — two new entries, one bug-fix to two mis-declared ones); NET-D1 (protocol 776 assertion), NET-D4 (state-machine traversal exercised end-to-end by a real client), NET-D6 (offline-mode for every automated run; the online-mode manual pass restated as a documented procedure), NET-D11 (the Status Response/Pong fields this blueprint's probe asserts); PLAN-D5 (this blueprint is the mechanism that measures M1's own acceptance criteria); `11-roadmap-milestones.md`'s M1 Acceptance Criteria 1–3, verbatim, mapped 1:1 onto this blueprint's report cases. |
| Crates touched | New `crates/testing/test-harness/` (`rc-test-harness`) and `crates/testing/paritybot/` (`rc-paritybot`) — both dev/test-only workspace members, added to `12-workspace-structure.md`'s WS-D2 Crate Manifest by this blueprint's own governance changeset (already reflected in that document as of this blueprint's derivation). `xtask` (extended: `m1_report.rs`, `path_guard.rs`'s `PROTECTED_PATHS` table, `main.rs`'s `Command` enum). `.github/workflows/ci.yml` (extended: one new nightly/manual job, `m1-acceptance`; **no change** to the existing `gates` job — see Context). New `docs/MANUAL-VERIFICATION-M1.md`. |
| Estimated scope | L |

## Goal & Done definition

Give the project its first real, agent-executable measurement of a milestone's acceptance criteria end to end: a raw-TCP Server-List-Ping probe that validates `Status Response` JSON fields against NET-D11/NET-D1; an azalea-based bot driver (TEST-D8) that performs a genuine Handshake→Login→Configuration→Play sequence against a real server and holds a stable idle session, reporting exactly when and why it fails if it does; a reusable in-process "scripted fake server" test double that lets both of the above be proven correct without a real `rusty-clanker-server` build (Tier 1, every PR); a `xtask m1-report` verb that drives both tools against a real, freshly-spawned `rusty-clanker-server` subprocess and emits a machine-readable, per-criterion pass/fail JSON matching `11-roadmap-milestones.md`'s three M1 acceptance criteria; and the CI wiring that runs a short, real-time (not accelerated) smoke instance of that report every night, plus a full, uncompressed 30-minute instance on manual trigger — the run whose green result actually closes M1 per TEST-D50/PLAN-D5.

Done when:

- [ ] `cargo build -p rc-test-harness -p rc-paritybot -p xtask --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset (all against the in-process scripted fake server, no real `rusty-clanker-server` build required) passes under `cargo nextest run -p rc-test-harness -p rc-paritybot`.
- [ ] `cargo run -p xtask -- path-guard` still exits 0 against this blueprint's own governance changeset (labeled accordingly, per Constraints).
- [ ] `cargo run -p xtask -- m1-report --help` prints usage with zero panics (CLI wiring compiles and parses); a full `m1-report` run against a real `rusty-clanker-server` is **not** required for this blueprint's own Tier-1 Done state — see Context, "What this blueprint's own CI gate proves vs. what M1's nightly job proves."
- [ ] `docs/MANUAL-VERIFICATION-M1.md` exists and documents AC3's one non-automatable step precisely enough for a human (or an agent with human-supervised interactive OAuth) to execute it without further guidance.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`, `path-guard`, `lint-tests`, `verify-fixtures` — all already-existing `gates`/`guardrails` jobs, unmodified, now additionally covering the two new crates automatically via `cargo nextest run --workspace`) green on both `ubuntu-24.04` and `windows-2025`, on a clean checkout (TEST-D50). The new nightly `m1-acceptance` job is **not** part of this blueprint's own Tier-1 Done gate (TEST-D37's own "nightly, not PR-blocking" rule) — its own first green run, once M1-B02/M1-B04/M1-B05 have also landed, is what closes M1 itself, not this blueprint.

## Context (self-contained)

### Scope: not yet TEST-D7's full two-server differential leg

TEST-D7 defines the *eventual* differential-harness shape: two server subprocesses (vanilla `server.jar` and Rusty Clanker), one scripted bot driving both, packet-stream and world-state comparison. At M1 there is only one server worth testing (a hardcoded superflat placeholder has no real content to diff against vanilla's actual generation — that comparison starts being meaningful at `M5`, and mechanic-level differential scenarios start at `M3`, per `11-roadmap-milestones.md`). This blueprint therefore builds the *bot-driver* half of TEST-D7's architecture now — a real protocol-776 client (azalea, TEST-D8) driving one server and asserting connection-lifecycle behavior — deliberately **without** the comparator half (TEST-D9/TEST-D10's packet/world-state diff, which needs a second server). `rc-paritybot`'s own API (Deliverables, below) is written so the *same* scenario-running code is reusable unmodified once a second server enters the picture at `M3`+: `run_idle_stability_scenario` takes one target `host:port` and returns a typed outcome: a future two-server comparator wraps two calls to the same function, one per server, and diffs the two `ScenarioOutcome`s — no rewrite required. This is also why the crate is named `rc-paritybot`, not `rc-m1-bot`: it is the same crate TEST-D1 already reserves for the eventual differential-scenario runner, not a throwaway.

### The vanilla-client stand-in: why azalea, restated concretely

`11-roadmap-milestones.md`'s M1 Acceptance Criterion 1 says "an unmodified vanilla Java Edition 26.2 client." A human manually launching the real Minecraft launcher for every PR (or every night) is exactly the human-in-the-loop dependency this project's binding zero-human-test-loop principle (`09-testing-quality.md`, "Agent-Executable Verification") forbids for routine verification. TEST-D8 already resolves this for the whole project: **azalea**, a from-scratch Rust implementation of the real Java Edition wire protocol (not a hand-rolled minimal test stub — a real client library other Minecraft server operators use specifically to catch server-side breakage automatically), is the automatable stand-in. Verified live as part of deriving this blueprint (August 2026): azalea's `main` branch documents support for **Minecraft 26.2**, matching this project's NET-D1 pin exactly (crates.io's last *tagged* release lags behind, at 1.21.11/protocol 774 — consistent with TEST-D8's own "tracking git main, not the crates.io release" framing; a version this project's own dependency table pins by exact commit `rev`, not a floating branch, per `12-workspace-structure.md`'s `[workspace.dependencies]` entry this blueprint's own governance changeset adds — see Deliverables).

Azalea's relevant public shape, verified against its current documentation:

```rust
// azalea::prelude — already re-exported the way azalea's own docs show:
use azalea::prelude::*;

// Building and starting a bot:
ClientBuilder::new()
    .set_handler(handle)                 // async fn(Client, Event, State) -> eyre::Result<()>
    .start(account, "host:port")         // ResolvableAddr accepts "host:port" strings directly
    .await;                              // resolves once the connection ends (see Constraints,
                                          // "start() retries forever" — this blueprint never calls
                                          // start() without an outer bounded timeout, below)

// Accounts:
azalea::Account::offline("bot_name")            // no Mojang/Microsoft involvement — every
                                                 // automated run in this blueprint uses this
azalea::Account::microsoft("email@example.com").await   // real interactive OAuth device-code
                                                          // flow — the ONE thing this blueprint
                                                          // never calls automatically (Constraints)

// Events relevant to this blueprint (a non-exhaustive subset of azalea::Event):
Event::Login        // connection reached Play state and the client's own login bookkeeping
                     // finished — fires once, after the server's Play "Login" packet
Event::Spawn         // the client's own player entity has spawned in the world (fires after
                     // Login; this blueprint treats Spawn, not merely Login, as
                     // AC1's "spawns into the superflat placeholder world")
Event::Disconnect(reason: Option<String>)   // the connection ended (server-initiated
                                             // Disconnect packet, or a transport error)
```

### `azalea::ClientBuilder::start`'s infinite-retry behavior — and why this blueprint never calls it unwrapped

Azalea's own documentation states plainly: "if the client can't join, it'll keep retrying forever until it can." That is the right default for a long-running bot, and the wrong one for a bounded acceptance check — a fake or real server that never accepts a connection must not hang this blueprint's own tests or CI forever. Every call this blueprint makes to `start(...)` is therefore wrapped in an outer `tokio::time::timeout` (Deliverables, `idle_stability::run_idle_stability_scenario`) — a timeout elapsing before any `Event::Login` was ever observed is reported as `ScenarioError::LoginTimeout`, distinct from a timeout elapsing *after* `Event::Login`/`Event::Spawn` (reported as whatever mid-session outcome the handler's own state last recorded — see the API below). This is this blueprint's own concrete resolution of a azalea-integration detail no planning document fixes, restated here so no later blueprint needs to rediscover it.

### Assumed server CLI surface

Neither M0-B01 nor M1-B01 gives `rusty-clanker-server` a real `main.rs` (M1-B01's own Deliverables add only a `src/net/` module). By the time this blueprint is implemented, M1-B05 (the Play-state placeholder-world spawn) necessarily has — a server that never listens on a socket cannot spawn a player into anything. This blueprint fixes the **one** external contract it needs from that binary, restated precisely so its own implementer can verify or add it in a small, clearly-scoped touch if it is not already exactly this shape:

```
rusty-clanker-server --bind <ip:port> [--offline]
```

- `--bind <ip:port>`: overrides the listen address (default, if omitted, may be anything — this blueprint always passes an explicit ephemeral port it has itself reserved, see `rc_test_harness::process::find_free_port` below, so the server's own default is never exercised by this blueprint).
- `--offline`: disables NET-D6's online-mode session validation for this process's lifetime (vanilla's own well-established "offline-mode... retained for local/LAN testing parity" behavior, restated by NET-D6 itself) — every automated test and CI job in this blueprint passes this flag; **no CI job ever omits it**, per the oracle/session-server boundary below.
- The binary must exit non-zero and print a diagnostic to stderr if it cannot bind the requested address, rather than silently listening elsewhere — `rc_test_harness::process::spawn_server` (below) treats "the reserved port never accepts a connection within `startup_timeout`" as the sole readiness signal, so a silent fallback-port bind would manifest as an opaque timeout rather than a clear error; if `rusty-clanker-server` does not yet behave this way, adding this behavior is this blueprint's own small, in-scope addition.

This blueprint deliberately does **not** ask for a MOTD/version-name override flag: AC2 only requires the Status Response JSON's fields to be *present and well-typed*, with the protocol number matching NET-D1's 776 exactly — it does not require asserting against a specific configured string, so the server's own default MOTD/version-name text (M1-B02's own `default_status_payload`, `"Rusty Clanker 26.2"` plus ASSET-D22's disclaimer) is sufficient and this blueprint's probe never depends on its exact content.

### Relationship to M1-B02's own status test — complementary, not duplicated

M1-B02 already ships its own acceptance test, `status_probe_returns_expected_json_and_ping_pong`, which its own Done-definition describes as reproducing "M1's milestone acceptance criterion 2 exactly." Reading that test closely: it drives `rusty-clanker-server::net::handle_new_connection` **in-process**, over a loopback `TcpStream` pair the test itself constructs via `tokio::net::TcpListener` — a real socket, but never a real, separately-spawned `rusty-clanker-server` OS process, since M1-B02 explicitly does not build that binary's `main.rs`/composition root. It is the correct, cheap, Tier-1 **unit-level** proof that the connection-handling *logic* (`read_handshake`/`serve_status`/`handle_new_connection`) is correct in isolation. This blueprint's own probe (`rc_test_harness::probe::probe_status`, driven by `xtask m1-report` against a real `ManagedServer` subprocess) is the complementary **integration/acceptance-level** proof that the actually-shipped `rusty-clanker-server` binary — real `main()`, real argument parsing, real socket bind, whatever composition-root code some other M1 blueprint adds on top of M1-B02's logic — serves the identical external contract correctly end to end, which is what AC2's own wording ("a raw TCP probe... confirms the Status Response JSON") most naturally describes and what TEST-D7's differential-harness architecture (external OS-subprocess testing, never in-process function calls) already establishes as this project's acceptance-level methodology. The two tests are deliberately not merged: a regression in one layer without the other (a working `handle_new_connection` that a broken `main.rs` never actually calls with the right arguments, or vice versa) is exactly the class of bug layered testing exists to catch, and TEST-D5's own "cheapest layer first" framing already establishes duplicating a cheap unit check at a more expensive acceptance layer as the correct discipline, not redundant effort.

### The oracle/vanilla-jar boundary at M1: none needed, restated

TEST-D38/TEST-D41/TEST-D48 govern the vanilla `server.jar` **oracle** — the second server TEST-D7's full differential leg diffs against. Per "Scope" above, this blueprint's automated tests never launch a second server, so **no oracle jar, and no `xtask setup-oracle` call, appears anywhere in this blueprint's Deliverables, Acceptance tests, or CI wiring.** This is a deliberate, restated absence, not an oversight: the oracle remains reserved for whichever blueprint first implements TEST-D7's real two-server comparator (`M3`'s redstone-differential corpus at the earliest). The one place a *real* Minecraft artifact appears in this blueprint's scope at all is the manual verification procedure below, which uses a real player account, not a jar.

**CI never talks to Mojang — restated as this blueprint's own binding rule.** Every automated connection this blueprint's tests or CI jobs make — probe, bot driver, fake-server self-tests, the nightly `m1-acceptance` job — targets a server started with `--offline` (above). No automated code path in this blueprint's Deliverables ever constructs `azalea::Account::microsoft(...)`, ever sets `--offline` to false, or ever causes `rusty-clanker-server` to call Mojang's `hasJoined` session-validation endpoint (NET-D6). The **only** exercise of real online-mode validation is `docs/MANUAL-VERIFICATION-M1.md`'s human-run procedure (AC3), matching `11-roadmap-milestones.md`'s own framing of that criterion as "this one step cannot be fully automated — it depends on a live third-party account."

### CI tier placement, and "compressed timescale where legitimate vs. real-time where required"

| Tier | What runs | Duration | Cadence |
|---|---|---|---|
| Tier 1 (PR-blocking, `gates`/`guardrails`, unmodified) | This blueprint's own self-tests — probe and bot driver against the in-process scripted fake server, **no real `rusty-clanker-server` build** | Each self-test's own idle window is a few seconds of synthetic, in-process keep-alive traffic (Acceptance tests, below) — not a claim about real server timing, purely proving the harness's own bookkeeping | Every PR, both OS legs — reached automatically via `cargo nextest run --workspace` (WS-D9), which now covers the two new crates with **zero `ci.yml` edits** |
| Tier 2 (nightly, new `m1-acceptance` job) | `xtask m1-report --mode smoke` against a real, freshly-built, `--offline` `rusty-clanker-server` | `idle_duration = 90s` at the **real, uncompressed** vanilla keep-alive cadence (15000 ms, restated below) — long enough to observe ~6 genuine keep-alive round trips, short enough to fit a nightly budget | Nightly cron, both OS legs |
| Manual/on-demand (`workflow_dispatch` input `mode: full`, same job) | `xtask m1-report --mode full` against the same kind of real server | `idle_duration = 1800s` (30 real minutes) — AC1's **literal** threshold, run at real time, never accelerated | Triggered deliberately once a maintainer believes M1 is complete — this run's green result is what TEST-D50/PLAN-D5 actually treat as closing M1, mirroring `M0-B08`'s own "soak" job / `xtask setup-oracle`'s one-time-consent pattern: a nightly *signal*, a manual *gate* |

"Compressed" never means an accelerated keep-alive cadence — a shortened cadence would test a vanilla client's tolerance for a scenario vanilla itself never produces, defeating the purpose of using a real client. What is legitimately compressed is **duration**: fewer real 15-second cycles (Tier 2's 90 s) versus AC1's full real 1800 s (the manual/`workflow_dispatch` run) — both paced identically, only one running longer. `rc-paritybot`'s own idle loop (Deliverables) never overrides or fast-forwards the server's keep-alive timer; it only waits, at real wall-clock speed, for `idle_duration`, watching for `Event::Disconnect`.

**What this blueprint's own CI gate proves vs. what M1's nightly job proves.** This blueprint's Tier-1 Done state (the checkbox list above) is satisfied entirely by the fake-server self-tests — it proves the *harness* is correct. It deliberately does **not** require a green `m1-acceptance` run, because that job's first real, meaningful, green result can only happen once M1-B02/M1-B04/M1-B05 (this blueprint's own prerequisites) are *also* merged and their combined behavior is exercised together for the first time — this blueprint's implementer cannot make that job pass single-handedly if any prerequisite still has a gap, and blocking this blueprint's own Done state on a job it cannot single-handedly guarantee would be a design mistake, not rigor. This mirrors M0-B08's own "Ordering note on the `soak` job" precedent exactly (a CI job wired now, whose own green run closes a *milestone* criterion later, not the blueprint that wired it).

### Fake-server protocol cheat sheet — exact fields, restated

Handshake and Status rows below are **not independent research** — they restate M1-B02's own already-landed, already-tested `rc_protocol::{handshake, status}` modules field-for-field (packet ids, struct shapes, the exact `StatusResponsePayload` JSON schema, including `serde(rename_all = "camelCase")`'s `enforcesSecureChat` key), which is now this project's authoritative source for both states, superseding independent wiki research. Every other row (Login/Configuration/Play) is restated from `docs/research/mc-26.2/02-network-protocol.md` (§3.2–3.13, the legally-consulted 26.2 reference, ASSET-D18(f)) cross-checked live against minecraft.wiki's current Java Edition protocol pages and, for the one packet with real field-count risk (Play "Login"), against **azalea's own current source** (`azalea-protocol`'s `ClientboundLogin`/`CommonPlayerSpawnInfo` structs) — the exact structures the real client this harness embeds will itself decode, which is the strongest possible correctness guarantee available short of testing against a real `rusty-clanker-server`. **Numeric packet IDs for Login/Configuration/Play below are a best-effort, wiki-sourced sanity check, not this blueprint's authoritative source** — `reports/packets.json` (NET-D9's own `--reports` output, already produced by `xtask fetch-data`/`xtask setup-oracle` and already named by the research doc itself as "the ground truth for packet-id-per-phase counts"), or M1-B04's own already-merged, concrete Rust packet catalog (`crates/protocol/src/{login.rs, configuration.rs}`) for Login/Configuration, or M1-B05's own already-merged `crates/server/src/play/packets.rs` for Play, is authoritative; if either disagrees with an ID below, that source wins and this table is wrong, not the other way around. Every field here uses M1-B01's own `rc_protocol::{VarInt, VarLong}` encoding and `wire.rs` primitive layouts (VarInt-length-prefixed UTF-8 strings, big-endian fixed-width integers) — no new wire primitive is introduced.

| State | Direction | Packet (wiki name) | ID | Fields, in order |
|---|---|---|---|---|
| Status | S→C | Status Response (`rc_protocol::status::StatusResponse`, M1-B02) | `0x00`, confirmed | `json: String` (the full `StatusResponsePayload` JSON blob, M1-B02's exact schema — `version.{name,protocol}`, `players.{max,online,sample?}`, `description` (`serde_json::Value`), `favicon?`, `enforcesSecureChat`; this blueprint's probe parses it with `serde_json`, already workspace-pinned) |
| Status | C→S | Ping Request (`rc_protocol::status::PingRequest`, M1-B02) | `0x01`, confirmed | `payload: i64` (raw 8-byte big-endian, **not** VarLong — matches vanilla's plain `Long` framing for this one field) |
| Status | S→C | Pong Response (`rc_protocol::status::PongResponse`, M1-B02) | `0x01`, confirmed | `payload: i64` (echoed verbatim) |
| Login | C→S | Login Start | `0x00`, best-effort | `name: String` (≤16 chars), `player_uuid: [u8; 16]` (raw, not VarInt-prefixed) |
| Login | S→C | Login Success (a.k.a. `ClientboundLoginFinishedPacket`, source cartography §3.7 step 4) | `0x02`, confirmed against M1-B04's own `rc_protocol::login::{LoginSuccess, LoginProfile, LoginProfileProperty}` | `uuid: [u8; 16]`, `username: String` (≤16), `properties: prefixed_array<{name: String, value: String, signature: Option<String>}>` (M1-B04's `LoginProfileProperty` shape exactly — **no** `is_signed` field; empty array for offline mode — this blueprint's fake server and the harness's assumed offline-mode server both always send zero properties), `session_id: [u8; 16]` (per the research doc's own §3.7 step 4: `ClientboundLoginFinishedPacket(profile, sessionId)`) |
| Login | C→S | Login Acknowledged | `0x03` | *(no fields — terminal packet, source cartography §3.6: triggers the Configuration handoff)* |
| Configuration | S→C | Known Packs | best-effort, verify against `packets.json` | `packs: prefixed_array<{namespace: String, id: String, version: String}>` — this blueprint's fake server always sends an **empty** array (zero known packs requested), which is a legal, minimal `SynchronizeRegistriesTask` negotiation per the research doc's §3.12 |
| Configuration | C→S | Known Packs | best-effort, verify against `packets.json` | Same shape; the fake server does not inspect the client's reply content, only that one arrives, before proceeding |
| Configuration | S→C | Finish Configuration | best-effort, verify against `packets.json` | *(no fields — terminal packet, triggers the Play handoff, source cartography §3.6)* |
| Configuration | C→S | Acknowledge Finish Configuration | best-effort, verify against `packets.json` | *(no fields)* |
| Play | S→C | Login (a.k.a. `ClientboundLogin`, verified against azalea's own current source, August 2026) | best-effort, verify against `packets.json` | `player_id: i32` (`#[var]` → `VarInt`), `hardcore: bool`, `levels: prefixed_array<Identifier>` (`Identifier` = `VarInt`-length-prefixed `"namespace:path"` string — this blueprint's fake server sends exactly one, `"minecraft:overworld"`), `max_players: i32` (`#[var]`), `chunk_radius: u32` (`#[var]` — "view distance"), `simulation_distance: u32` (`#[var]`), `reduced_debug_info: bool`, `show_death_screen: bool` (send `true`), `do_limited_crafting: bool` (send `false`), then inline (not length-prefixed — a nested struct, not a collection) `common: CommonPlayerSpawnInfo` = `dimension_type` (VarInt registry reference — this blueprint's fake server sends `VarInt(0)`, valid only because exactly one dimension-type registry entry was ever advertised; **verify this sub-field's exact wire encoding against azalea-protocol's own `AzBuf`-derived (de)serialization at implementation time**, since no planning document pins registry-holder wire encoding yet), `dimension: Identifier` (send `"minecraft:overworld"`), `seed: i64` (send `0`), `game_type: u8` (send `0` = Survival), `previous_game_type: i8` (send `-1` = "no previous"), `is_debug: bool` (`false`), `is_flat: bool` (`true` — matches the superflat placeholder), `has_death_location: bool` (`false`, no following optional fields), `portal_cooldown: u32` (`#[var]`, send `0`), `sea_level: i32` (`#[var]`? — **verify against azalea-protocol's own encoding**; send `64`), then back in the outer packet: `online_mode: bool` (`false`), `enforces_secure_chat: bool` (`false`) |
| Play | S→C | Keep Alive | best-effort, verify against `packets.json` | `id: i64` (raw 8-byte big-endian, **not** VarLong — matches azalea's own `ClientboundKeepAlive { id: u64 }`, no `#[var]` attribute observed) |
| Play | C→S | Keep Alive | best-effort, verify against `packets.json` | `id: i64` (echoed) — azalea answers this automatically as part of its own connection bookkeeping; this blueprint's fake server never needs to hand-construct this direction for any self-test in this blueprint (see Acceptance tests — no self-test asserts on the client's keep-alive replies, only on whether the *connection itself* survives the idle window) |

Real vanilla keep-alive cadence, restated from the research doc's §3.10/§5: the server sends a fresh `Keep Alive` every **15000 ms** of connection idle time; a real vanilla server disconnects a client whose reply is overdue. This blueprint's fake server, for its one idle-stability self-test that must actually hold the connection open (Acceptance tests, below), sends `Keep Alive` packets on this same real cadence — never an accelerated one, consistent with the "compressed timescale" rule above.

### Resolved: two `PROTECTED_PATHS` gaps found while deriving this blueprint

M0-B08's `path_guard::PROTECTED_PATHS` table pre-declared rows for `crates/testing/rc-test-harness/**` and `crates/testing/rc-paritybot/src/**` — both written using the crate's *name* as its directory segment. `12-workspace-structure.md`'s WS-D1 naming convention is explicit that a crate's directory drops the `rc-` prefix (`rc-core` → `crates/core/`); this blueprint's own crates therefore live at `crates/testing/test-harness/` and `crates/testing/paritybot/` (already reflected in that document's Repository Layout and Crate Manifest, updated as part of deriving this blueprint). As pre-declared, neither existing pattern would ever match a real path this blueprint creates — a live gap, not a hypothetical one, the moment this blueprint's files exist. Separately, `rc-paritybot`'s row was scoped to `src/**` only, leaving its own `tests/` directory unprotected (the generic `crates/*/tests/**` row does not match either, since `*` matches exactly one path segment and `crates/testing/paritybot/tests/**` is one segment deeper than that pattern's shape allows). This blueprint's governance changeset corrects both: it replaces the two mis-named rows with `crates/testing/test-harness/**` and `crates/testing/paritybot/**` (the latter now covering `src/`, `tests/`, and any future subdirectory uniformly, matching pattern #8's own broader style) — see Deliverables, `path_guard.rs`.

## Deliverables

### `crates/testing/test-harness/Cargo.toml` (new)

```toml
[package]
name = "rc-test-harness"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
serde_json = { workspace = true }
thiserror = { workspace = true }

[[bin]]
name = "status_probe"
path = "src/bin/status_probe.rs"
```

(No `tokio` dependency — every deliverable in this crate is deliberately synchronous `std`-only I/O; see Context, "rc-test-harness stays synchronous.")

### `crates/testing/test-harness/src/lib.rs`

```rust
//! `rc-test-harness` — dev/test-only (TEST-D1): subprocess orchestration for a
//! `rusty-clanker-server` under test (`process`), the raw-TCP Server-List-Ping status
//! probe (`probe`, also exposed as the `status_probe` binary), and the in-process
//! scripted "fake server" test double (`fake_server`) both this crate's own tests and
//! `rc-paritybot`'s tests drive a real protocol client against. World-state
//! hashing/diffing and the synchronous test-mode tick driver (TEST-D1's other named
//! responsibilities for this crate) are reserved, unimplemented, for the milestone
//! that first needs real comparable world content (M2+) — not part of this blueprint.

pub mod fake_server;
pub mod probe;
pub mod process;
```

### `crates/testing/test-harness/src/process.rs`

```rust
use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Child;
use std::time::Duration;

/// Binds `127.0.0.1:0`, reads the OS-assigned port, then immediately drops the
/// listener — a standard reserve-then-release free-port allocation. A small race
/// (another process claiming the port before `spawn_server` gets to bind it) is
/// accepted as a rare CI flake risk, not designed around further.
pub fn find_free_port() -> io::Result<u16>;

pub struct ManagedServerConfig {
    pub binary_path: PathBuf,
    /// Passed as `--offline` when true. Every caller in this blueprint's own
    /// Deliverables always passes `true` — see Context's oracle-boundary rule.
    pub offline: bool,
    pub startup_timeout: Duration,   // default helper: Duration::from_secs(30)
    pub extra_args: Vec<String>,
}

/// An owned, running `rusty-clanker-server` subprocess bound to `addr`. Dropping this
/// value always kills the child process (best-effort `Child::kill`, errors ignored) —
/// guaranteed teardown even if a caller returns early or panics mid-test.
pub struct ManagedServer {
    child: Child,
    pub addr: SocketAddr,
}

impl Drop for ManagedServer {
    fn drop(&mut self);
}

#[derive(Debug, thiserror::Error)]
pub enum SpawnError {
    #[error("failed to reserve a free port: {0}")]
    PortReservation(io::Error),
    #[error("failed to spawn {path}: {source}")]
    Spawn { path: String, source: io::Error },
    #[error("server did not accept a connection on {addr} within {elapsed:?}")]
    StartupTimeout { addr: SocketAddr, elapsed: Duration },
}

/// Reserves a free port (`find_free_port`), spawns `binary_path --bind
/// 127.0.0.1:<port> [--offline]`, then polls a raw TCP connect attempt against that
/// port (100 ms interval) until one succeeds or `startup_timeout` elapses — the sole
/// readiness signal (Context, "Assumed server CLI surface"). On a startup timeout the
/// child process is killed before returning the error.
pub fn spawn_server(config: ManagedServerConfig) -> Result<ManagedServer, SpawnError>;
```

### `crates/testing/test-harness/src/probe.rs`

```rust
use std::io;
use std::net::TcpStream;
use std::time::Duration;

pub struct ProbeConfig {
    pub host: String,
    pub port: u16,
    pub connect_timeout: Duration,   // default helper: Duration::from_secs(5)
}

/// The fields AC2 requires be present and well-typed: protocol number, version name,
/// online/max player counts, MOTD (the `description` field of the JSON blob — a raw
/// text-component value, kept as an opaque `serde_json::Value` rather than parsed
/// into a typed component tree, since no packet catalog/text-component type exists
/// yet at M1 — a later blueprint may replace this with a typed value without changing
/// this struct's other fields).
#[derive(Debug, Clone)]
pub struct ProbeResult {
    pub protocol_version: i64,
    pub version_name: String,
    pub motd: serde_json::Value,
    pub online_players: i64,
    pub max_players: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("connect failed: {0}")]
    Connect(io::Error),
    #[error("connect/read timed out after {0:?}")]
    Timeout(Duration),
    #[error("frame decode error: {0}")]
    Frame(String),
    #[error("status JSON is not valid JSON: {0}")]
    MalformedJson(String),
    #[error("status JSON is missing required field `{0}`")]
    MissingField(&'static str),
    #[error("protocol version mismatch: expected {expected}, server reports {actual}")]
    ProtocolMismatch { expected: i64, actual: i64 },
}

/// Performs one full, single-shot Server List Ping: Handshake (Intention=Status) →
/// Status Request → Status Response → Ping Request → Pong Response, over a plain,
/// unencrypted, uncompressed connection (matching NET-D5's own "status is single-shot,
/// never touches compression negotiation" framing) — entirely synchronous `std::net`
/// I/O, no tokio runtime needed. Validates the decoded JSON's `version.protocol`
/// against `expected_protocol` and that `version.name`, `players.online`,
/// `players.max`, and `description` are all present with the expected JSON types,
/// returning the first `ProbeError` encountered. A connection or read exceeding
/// `config.connect_timeout` is `ProbeError::Timeout`, never a hang.
pub fn probe_status(config: &ProbeConfig, expected_protocol: i64) -> Result<ProbeResult, ProbeError>;
```

### `crates/testing/test-harness/src/bin/status_probe.rs`

```rust
//! Standalone raw-TCP status-probe binary (NET-D11, M1 Acceptance Criterion 2). Not a
//! Minecraft client — deliberately reuses none of `rc_protocol`'s packet-catalog
//! machinery beyond the framing/VarInt primitives `probe::probe_status` calls
//! directly, matching AC2's own "a raw TCP probe (not a Minecraft client)" wording.
//!
//! Usage: `status_probe <host> <port> <expected_protocol>`
//! Exit code 0 and a one-line human-readable summary to stdout on success; nonzero
//! and the `ProbeError`'s own message to stderr on failure. No `clap` dependency (not
//! workspace-pinned for non-`xtask` crates) — three positional arguments, hand-parsed.
fn main() -> std::process::ExitCode;
```

### `crates/testing/test-harness/src/fake_server.rs`

```rust
use std::net::{SocketAddr, TcpListener};
use std::thread::JoinHandle;
use std::time::Duration;

/// One scripted step. Every self-test in this blueprint (its own and
/// `rc-paritybot`'s) builds a `Vec<ScriptStep>` and hands it to `spawn`. Steps that
/// `Expect*` a client packet read and validate only what this blueprint's fake server
/// needs to proceed (e.g. `ExpectLoginStart` reads and discards the name/UUID rather
/// than asserting a specific value) — the fake server is a permissive stand-in for a
/// real server's request side, strict only where a self-test specifically wants a
/// negative case (`SendMalformed*` steps).
pub enum ScriptStep {
    ExpectHandshake,                          // reads Handshake, discards fields, does not
                                               // validate `Intention` (both Status- and
                                               // Login-flow scripts start with this step)
    ExpectStatusRequest,
    SendStatusResponse { json: String },       // caller controls the exact JSON, including
                                                // deliberately malformed/incomplete bodies
                                                // for negative self-tests
    ExpectPingRequest,
    SendPongEcho,
    ExpectLoginStart,
    SendLoginSuccess { username: String },
    ExpectLoginAcknowledged,
    ExpectClientInformation,                   // configuration phase's first client packet;
                                                // read and discarded
    SendKnownPacksEmpty,
    ExpectKnownPacksResponse,
    SendFinishConfiguration,
    ExpectAcknowledgeFinishConfiguration,
    SendPlayLogin,                              // the full ClientboundLogin per the
                                                 // cheat sheet, fixed placeholder field
                                                 // values baked into this step's own
                                                 // implementation — no per-call
                                                 // parameterization needed by any self-test
                                                 // in this blueprint
    RunIdleFor { duration: Duration, keepalive_interval: Duration },  // sends real
                                                 // `Keep Alive` packets on `keepalive_interval`
                                                 // for `duration`, ignoring the client's own
                                                 // keep-alive replies (azalea answers them
                                                 // automatically; this fake server does not
                                                 // need to read them to prove connection
                                                 // survival — only that it is *still connected*
                                                 // at the end)
    CloseAbruptly,                              // drops the TCP connection with no Disconnect
                                                 // packet — the harness-side failure this
                                                 // blueprint's negative self-tests assert on
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FakeServerOutcome {
    ScriptCompleted,
    UnexpectedClientClose { at_step: usize },
    IoError { at_step: usize, message: String },
}

/// Binds an ephemeral loopback port, spawns a background OS thread that accepts
/// exactly one connection and executes `script` step by step (blocking `std::net`
/// I/O throughout — no tokio dependency in this crate), and returns the bound address
/// plus a `JoinHandle` the caller joins after its own client-side interaction
/// completes. `CloseAbruptly` and reaching the script's end both terminate the
/// thread; any `Expect*` step reading a mismatched or absent packet where the
/// connection has already closed reports `UnexpectedClientClose` naming the step
/// index, not a panic.
pub fn spawn(script: Vec<ScriptStep>) -> (SocketAddr, JoinHandle<FakeServerOutcome>);
```

### `crates/testing/paritybot/Cargo.toml` (new)

```toml
[package]
name = "rc-paritybot"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-test-harness = { path = "../test-harness" }
tokio = { workspace = true }
azalea = { workspace = true }   # TEST-D8 — git dependency, see 12-workspace-structure.md
thiserror = { workspace = true }

[dev-dependencies]
# none beyond the above — this crate's own self-tests use rc-test-harness's
# fake_server directly, no additional test-only dependency needed
```

### `crates/testing/paritybot/src/lib.rs`

```rust
//! `rc-paritybot` — dev/test-only (TEST-D1/TEST-D8): the azalea-based bot driver.
//! `idle_stability` is this blueprint's own scenario; a future two-server comparator
//! (TEST-D9/TEST-D10, starting at M3+) wraps this same module's function twice, once
//! per server, rather than replacing it.

pub mod idle_stability;
```

### `crates/testing/paritybot/src/idle_stability.rs`

```rust
use std::time::Duration;

pub struct ScenarioConfig {
    pub host: String,
    pub port: u16,
    pub username: String,          // passed to `azalea::Account::offline` — every
                                    // automated caller in this blueprint uses an
                                    // offline account (Context, oracle boundary)
    pub login_timeout: Duration,   // default helper: Duration::from_secs(30) — matches
                                    // vanilla's own MAX_TICKS_BEFORE_LOGIN watchdog
                                    // (600 ticks @ 20 TPS, research doc §5)
    pub idle_duration: Duration,   // Tier-2 smoke: 90s; manual/full: 1800s (Context)
}

#[derive(Debug, Clone)]
pub struct ScenarioOutcome {
    pub reached_login: bool,
    pub reached_spawn: bool,
    /// `Some(d)` iff a disconnect was observed at all, `d` measured from the scenario's
    /// own start — `None` means the connection survived the full `idle_duration` and
    /// this function itself performed a clean client-initiated disconnect at the end.
    pub disconnected_at: Option<Duration>,
    pub disconnect_reason: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ScenarioError {
    #[error("no Event::Login observed within the {0:?} login timeout")]
    LoginTimeout(Duration),
    #[error("disconnected before Event::Spawn (after Event::Login): {reason:?}")]
    DisconnectedBeforeSpawn { reason: Option<String> },
    #[error("disconnected during the idle window at {after:?} (of {expected:?}): {reason:?}")]
    DisconnectedDuringIdle { after: Duration, expected: Duration, reason: Option<String> },
}

/// Runs one idle-stability scenario against `config.host:config.port`: connects with
/// `azalea::Account::offline(config.username)`, waits (bounded by
/// `config.login_timeout`, wrapping the whole `ClientBuilder::start` call per
/// Context's "start() retries forever" note) for `Event::Login` then `Event::Spawn`,
/// then holds the connection open for exactly `config.idle_duration` of real
/// wall-clock time, watching for `Event::Disconnect` throughout, then performs a
/// clean client-side disconnect and returns. Any disconnect observed before
/// `idle_duration` elapses is `Err(ScenarioError::DisconnectedBeforeSpawn)` (if
/// before `Event::Spawn`) or `Err(ScenarioError::DisconnectedDuringIdle)` (after).
/// `Event::Login` never observed within `login_timeout` is
/// `Err(ScenarioError::LoginTimeout)`.
pub async fn run_idle_stability_scenario(config: ScenarioConfig) -> Result<ScenarioOutcome, ScenarioError>;
```

### `xtask/src/m1_report.rs` (new)

```rust
use crate::tier_result::TierResult;
use std::time::Duration;

#[derive(serde::Serialize)]
pub struct ManualStep {
    pub id: &'static str,
    pub description: &'static str,
    pub procedure_doc: &'static str,
}

/// Wraps `TierResult` (unmodified — see Constraints, "no edit to tier_result.rs")
/// with the one field TEST-D40's schema has no slot for: a manual, non-automatable
/// step (AC3), which is never a `CaseResult` and never affects `automated.status`.
#[derive(serde::Serialize)]
pub struct M1ReportResult {
    #[serde(flatten)]
    pub automated: TierResult,   // tier = "m1-acceptance"; cases named "AC1a_status_pong",
                                  // "AC1b_login_config_play_spawn", "AC1c_idle_stability",
                                  // "AC2_status_json_fields" — every case Pass/Fail per
                                  // tier_result::Status, aggregated the same way tier1::run
                                  // already aggregates (Status::Fail if any case failed)
    pub manual_steps: Vec<ManualStep>,   // always exactly one entry, AC3
    pub mode: String,                     // "smoke" | "full"
    pub target: String,                   // "<ip>:<port>" actually used
}

pub const OUT_PATH: &str = "target/verify/m1-acceptance.json";

/// CLI entry point (`xtask m1-report --server-bin <path> --mode {smoke|full}`):
/// resolves `idle_duration` from `mode` (`smoke` → 90s, `full` → 1800s; both use the
/// real, uncompressed keep-alive cadence — Context), spawns `rusty-clanker-server`
/// via `rc_test_harness::process::spawn_server` (`--offline` always set), runs
/// `rc_test_harness::probe::probe_status` once (feeding both the `AC1a`/`AC2` cases),
/// then `rc_paritybot::idle_stability::run_idle_stability_scenario` once (feeding
/// `AC1b`/`AC1c`) inside a `tokio::runtime::Runtime::new()?.block_on(...)` (xtask's
/// own `main` stays synchronous — this is the only verb that touches an async
/// runtime, isolated here), tears the server down (`ManagedServer`'s `Drop`), builds
/// and writes `M1ReportResult` to `OUT_PATH`, returns the matching `ExitCode`
/// (`SUCCESS` iff `automated.status == Status::Pass` — the manual step never gates
/// this verb's own exit code, per Context/PLAN-D5).
pub fn run(server_bin: std::path::PathBuf, mode: Mode) -> std::process::ExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Mode { Smoke, Full }

impl Mode {
    /// `Smoke` → `Duration::from_secs(90)`, `Full` → `Duration::from_secs(1800)`.
    pub fn idle_duration(self) -> Duration;
}
```

### `xtask/src/path_guard.rs` (modify — two-row correction, see Context)

Replace the two mis-declared rows (originally `crates/testing/rc-test-harness/**` and `crates/testing/rc-paritybot/src/**`) in the `PROTECTED_PATHS` table with:

```rust
ProtectedPath { pattern: "crates/testing/test-harness/**", reason: "rc-test-harness: process orchestration, probe, fake-server logic (M1-B06)" },
ProtectedPath { pattern: "crates/testing/paritybot/**", reason: "rc-paritybot: bot-driver scenario logic (M1-B06) — covers src/ and tests/ uniformly" },
```

No other row changes; the table's total entry count is unchanged (a replace, not an addition).

### `xtask/src/main.rs` (modify — one new `Command` variant, extending M0-B01/M0-B08's enum unchanged otherwise)

```rust
/// M1-B06: drives the M1 acceptance harness against a real, freshly-spawned
/// `rusty-clanker-server` and writes `target/verify/m1-acceptance.json`.
M1Report {
    #[arg(long)]
    server_bin: std::path::PathBuf,
    #[arg(long, value_enum, default_value_t = m1_report::Mode::Smoke)]
    mode: m1_report::Mode,
},
```

One new `match` arm calling `m1_report::run(server_bin, mode)`. `xtask/Cargo.toml` gains `tokio = { workspace = true }` (needed only by `m1_report.rs`'s own `block_on`) and two new path dependencies:

```toml
rc-test-harness = { path = "../crates/testing/test-harness" }
rc-paritybot = { path = "../crates/testing/paritybot" }
```

### `.github/workflows/ci.yml` (modify — one new job appended; `gates`/`guardrails` untouched)

```yaml
on:
  push:
  pull_request:
  workflow_dispatch:
    inputs:
      m1_report_mode:
        description: "m1-acceptance report mode when manually triggered"
        type: choice
        options: [smoke, full]
        default: smoke
  schedule:
    - cron: "0 7 * * *"
    # (M0-B08's existing `soak` cron entry is unchanged; this is the same trigger
    # block, now also gating the new `m1-acceptance` job below alongside `soak`)

jobs:
  # ... existing `gates` and `guardrails` jobs, byte-for-byte unchanged ...
  # ... existing `soak` job, byte-for-byte unchanged ...

  m1-acceptance:
    name: m1-acceptance (${{ matrix.os }})
    # Nightly cron: always `smoke`. Manual dispatch: whatever `m1_report_mode` was
    # chosen (default `smoke`) — the real, uncompressed 30-minute `full` run is
    # deliberately never on the automatic nightly cadence (Context: manual/on-demand
    # is the one that closes M1, not a routine nightly cost).
    if: github.event_name == 'schedule' || github.event_name == 'workflow_dispatch'
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-24.04, windows-2025]
    steps:
      - uses: actions/checkout@v4
      - name: Install pinned toolchain (rust-toolchain.toml)
        run: rustup show
      - uses: Swatinem/rust-cache@v2
      - name: Build rusty-clanker-server (monolithic)
        run: cargo build --release -p rusty-clanker-server --no-default-features --features monolithic
      - name: m1-report
        shell: bash
        run: |
          MODE="${{ github.event_name == 'workflow_dispatch' && inputs.m1_report_mode || 'smoke' }}"
          cargo run -p xtask -- m1-report --server-bin target/release/rusty-clanker-server --mode "$MODE"
      - name: Upload m1-acceptance report
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: m1-acceptance-${{ matrix.os }}
          path: target/verify/m1-acceptance.json
          if-no-files-found: warn
```

### `docs/MANUAL-VERIFICATION-M1.md` (new)

```markdown
# M1 Manual Verification — Online-Mode Session Validation (Acceptance Criterion 3)

This is the one M1 acceptance step this project's own binding rules forbid automating
(`09-testing-quality.md`'s zero-human-test-loop principle governs *routine*
verification; a real Microsoft/Mojang account's login flow is a genuine one-time
human action, not a routine check). Perform it once per M1 completion attempt,
immediately after a `full`-mode `m1-acceptance` CI run (`.github/workflows/ci.yml`)
is green.

## Procedure

1. Start a `rusty-clanker-server` build **without** `--offline` (online-mode is the
   documented default, NET-D6), bound to a reachable address.
2. Either:
   - **(a)** Launch the real, unmodified vanilla Java Edition 26.2 client via the
     official Minecraft launcher, using a genuine purchased Microsoft account, and
     connect to the server; or
   - **(b)** Run `cargo run -p rc-paritybot --example manual_online_check -- <host> <port> <email>`
     (a small, interactive-only example this blueprint does not wire into any
     automated test) — this calls `azalea::Account::microsoft(email).await`, which
     opens a real interactive Microsoft device-code OAuth flow in your terminal.
3. Confirm the connection succeeds (spawns into the world, no `unverified_username`/
   `authservers_down` disconnect) — this is the direct, positive proof that
   `rusty-clanker-server`'s NET-D6 `hasJoined` call against Mojang's real session
   server succeeded for a genuine account.
4. Record the date, the account used (username only, never credentials), and the
   engine build/commit hash tested, in the M1 completion record wherever this
   project tracks milestone sign-off.

Never automate this procedure. Never store or transmit account credentials as part of
any script this project ships.
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** this blueprint's test-authoring changeset (`Changeset-Type: test-authoring`) is every file under `crates/testing/test-harness/tests/`, `crates/testing/paritybot/tests/`, plus every `crates/testing/{test-harness,paritybot}/src/*.rs` module from Deliverables with every function body `todo!()`-stubbed. The governance changeset (Implementation steps, below) fills in real bodies and adds `xtask`'s/`ci.yml`'s/the manual-doc's own files; it must not modify any file in this list. Per Context's "two `PROTECTED_PATHS` gaps," this test-authoring changeset is the **first** commit under which `crates/testing/test-harness/**`/`crates/testing/paritybot/**` exist at all — `path-guard` correctly permits it as a `test-authoring`-labeled changeset regardless.

### `crates/testing/test-harness/tests/probe_self_tests.rs`

1. `probe_passes_against_wellformed_status_server` — `fake_server::spawn` with `[ExpectHandshake, ExpectStatusRequest, SendStatusResponse { json: <a well-formed ServerStatus JSON literal with protocol=776, version name "Rusty Clanker 0.1.0 (26.2)", players.online=0, players.max=20, description="A Rusty Clanker Server"> }, ExpectPingRequest, SendPongEcho]`; `probe::probe_status(&config_pointing_at_the_returned_addr, 776)` → `Ok(ProbeResult { protocol_version: 776, .. })` with every field matching the literal above.
2. `probe_fails_on_protocol_mismatch` — same script but `json`'s `version.protocol` is `775`; `probe_status(.., 776)` → `Err(ProbeError::ProtocolMismatch { expected: 776, actual: 775 })`.
3. `probe_fails_on_malformed_json` — `SendStatusResponse { json: "{not valid json".into() }` → `Err(ProbeError::MalformedJson(_))`.
4. `probe_fails_on_missing_players_field` — a well-formed-JSON status body with the entire `players` key omitted → `Err(ProbeError::MissingField("players"))` (or the specific sub-field name, implementer's choice, as long as it names `players`-related absence).
5. `probe_fails_on_connection_refused` — `probe_status` against `127.0.0.1:<a port with nothing listening>` (e.g. `find_free_port` then immediately drop without ever spawning a fake server) → `Err(ProbeError::Connect(_))`, returns within the configured `connect_timeout`, does not hang the test process.

### `crates/testing/test-harness/tests/process_self_tests.rs`

1. `find_free_port_returns_a_bindable_port` — call `find_free_port`, then successfully `TcpListener::bind(("127.0.0.1", returned_port))` immediately afterward (accepting the documented small race as a non-blocking risk).
2. `spawn_server_reports_startup_timeout_for_a_binary_that_never_listens` — `ManagedServerConfig` pointing `binary_path` at a trivial always-exits-immediately test fixture binary (e.g. `std::env::current_exe()`'s own test harness binary invoked with an arg it doesn't recognize, or any locally-available always-fast-exiting executable — implementer's choice of a portable fixture, documented in a code comment) with `startup_timeout: Duration::from_millis(500)` → `Err(SpawnError::StartupTimeout { .. })` within a bounded wall-clock margin of that timeout (asserted via a test-level `Instant` check, generous margin, e.g. `< 2s` total).

### `crates/testing/test-harness/tests/fake_server_self_tests.rs`

1. `full_handshake_status_script_completes` — spawn `fake_server` with the Status-phase script from `probe_self_tests.rs`'s case 1, drive it with a plain `TcpStream` performing the matching client-side sequence by hand (using `rc_protocol`'s own `frame`/`VarInt`/`wire` primitives directly — this is the one test in this blueprint that exercises the fake server's `Expect*` steps without going through `probe::probe_status`, proving the script executor itself, independent of the probe's own correctness) → the joined `FakeServerOutcome` is `ScriptCompleted`.
2. `unexpected_close_reports_the_failing_step_index` — a script `[ExpectHandshake, ExpectStatusRequest, ...]` where the test's own client-side driver closes its socket immediately after the handshake, before sending `Status Request` → `FakeServerOutcome::UnexpectedClientClose { at_step: 1 }` (index of the `ExpectStatusRequest` step that never completed).

### `crates/testing/paritybot/tests/idle_stability_self_tests.rs`

(Every case below uses `rc_test_harness::fake_server::spawn` with the full Login→Configuration→Play script from the cheat sheet, `RunIdleFor { keepalive_interval: Duration::from_millis(200), .. }` — a real, honest cadence for a **synthetic, self-test-only** connection, distinct from the real 15000 ms cadence a fake server representing a *real server's timing* would use; this blueprint's only claim about the real 15000 ms cadence is Tier 2/manual's runs against the real binary, never these self-tests. `ScenarioConfig`'s `login_timeout` in every case below is `Duration::from_secs(5)`, generous for an all-loopback, all-synchronous fake server.)

1. `reaches_spawn_and_survives_the_full_idle_window` — script ends with `RunIdleFor { duration: Duration::from_secs(2), keepalive_interval: Duration::from_millis(200) }` then the thread returns `ScriptCompleted` (no `CloseAbruptly`); `run_idle_stability_scenario` with `idle_duration: Duration::from_secs(2)` → `Ok(ScenarioOutcome { reached_login: true, reached_spawn: true, disconnected_at: None, .. })`.
2. `reports_disconnected_before_spawn` — script is `[..., SendLoginSuccess { .. }, CloseAbruptly]` (never reaches `SendPlayLogin`) → `Err(ScenarioError::DisconnectedBeforeSpawn { .. })`.
3. `reports_disconnected_during_idle` — script reaches `SendPlayLogin` then `RunIdleFor { duration: Duration::from_millis(500), .. }` then `CloseAbruptly`, while `run_idle_stability_scenario`'s own `idle_duration` is `Duration::from_secs(2)` (longer than the fake server holds the connection) → `Err(ScenarioError::DisconnectedDuringIdle { after, expected, .. })` with `after` in `450ms..1000ms` (a generous window around the fake server's own 500 ms hold, accounting for scheduling jitter) and `expected == Duration::from_secs(2)`.
4. `reports_login_timeout_when_server_never_responds` — `fake_server::spawn` with a script of `[ExpectHandshake]` only (the fake server accepts the TCP connection but never sends `Login Success`, never mind reaching Play) → `run_idle_stability_scenario` with `login_timeout: Duration::from_millis(500)` → `Err(ScenarioError::LoginTimeout(Duration::from_millis(500)))`, returning within a generous margin of 500 ms (not hanging on azalea's own infinite-retry `start()` behavior — Context).

### `xtask/tests/m1_report_cli.rs`

1. `mode_idle_duration_smoke_is_90s` — `Mode::Smoke.idle_duration() == Duration::from_secs(90)`.
2. `mode_idle_duration_full_is_1800s` — `Mode::Full.idle_duration() == Duration::from_secs(1800)`.
3. `m1_report_result_serializes_with_flattened_tier_fields` — build an `M1ReportResult` with a passing `TierResult` (`tier: "m1-acceptance"`) and one `ManualStep`, serialize to `serde_json::Value`, assert the top-level object has `tier`, `status`, `cases` (from the flattened `TierResult`, per TEST-D40's existing schema — unmodified) **and** `manual_steps`, `mode`, `target` as sibling keys.
4. `path_guard_protects_the_corrected_testing_crate_paths` — `path_guard::check_paths(ChangesetType::Implementation, &["crates/testing/test-harness/src/probe.rs".into(), "crates/testing/paritybot/tests/idle_stability_self_tests.rs".into()])` → exactly 2 violations (proving both corrected rows actually match real M1-B06 paths, closing the gap Context describes).

## Implementation steps

1. **`crates/testing/test-harness/Cargo.toml`, `src/lib.rs`.** Create exactly as specified. Observable: `cargo build -p rc-test-harness` succeeds (empty/stubbed modules).
2. **`process.rs`.** Implement `find_free_port` (`TcpListener::bind(("127.0.0.1", 0))`, read `.local_addr()?.port()`, drop). Implement `spawn_server`: reserve a port, build a `std::process::Command` for `binary_path` with `["--bind", &format!("127.0.0.1:{port}")]` plus `["--offline"]` if `config.offline` plus `config.extra_args`, spawn, then loop `TcpStream::connect_timeout` against the reserved port at 100 ms intervals until success or `startup_timeout` elapses (killing the child and returning `SpawnError::StartupTimeout` on the latter). Implement `Drop for ManagedServer`. Observable: `process_self_tests.rs` passes.
3. **`probe.rs`.** Implement `probe_status` using `rc_protocol`'s `frame`/`varint`/`wire` primitives directly (M1-B01's public API) to hand-encode Handshake/Status Request/Ping Request and hand-decode Status Response/Pong Response per the cheat sheet's Status-phase rows; parse the decoded JSON string with `serde_json::from_str`, validate/extract fields, map absent/mistyped fields to `ProbeError::MissingField`/`MalformedJson`, compare `version.protocol` against `expected_protocol`. Wrap the whole connect-through-pong sequence in a `connect_timeout`-bounded read/write pattern (`TcpStream::set_read_timeout`/`set_write_timeout`). Observable: `probe_self_tests.rs` passes.
4. **`src/bin/status_probe.rs`.** Hand-parse `std::env::args()` (host, port, expected_protocol), call `probe::probe_status`, print a one-line summary or the error, return the matching `ExitCode`. Observable: `cargo run -p rc-test-harness --bin status_probe -- 127.0.0.1 0 776` exits nonzero with a clear "connect failed" message (nothing listening on port 0 as a literal target is itself a valid negative smoke check).
5. **`fake_server.rs`.** Implement `ScriptStep`'s `Send*`/`Expect*` handling using `rc_protocol`'s framing/wire primitives directly (same toolset as `probe.rs`) for every step through `SendPlayLogin` (per the cheat sheet's full field list, including the `CommonPlayerSpawnInfo` sub-fields — verify the two flagged sub-field encodings, `dimension_type`/`sea_level`, against `azalea-protocol`'s own current `AzBuf`-derived (de)serialization before finalizing this step's byte output, per Context). Implement `RunIdleFor` as a loop sending `Keep Alive` at `keepalive_interval` for `duration`, never blocking on a client reply. Implement `spawn`'s background-thread accept-and-execute loop and `FakeServerOutcome` reporting. Observable: `fake_server_self_tests.rs` passes.
6. **`crates/testing/paritybot/Cargo.toml`, `src/lib.rs`.** Create exactly as specified (including the `azalea` git dependency, pinned to a real commit `rev` resolved at this step — record the exact commit and Minecraft-version tag it reports supporting in a code comment next to the dependency line). Observable: `cargo build -p rc-paritybot` succeeds.
7. **`idle_stability.rs`.** Implement `run_idle_stability_scenario`: build `ClientBuilder::new().set_handler(handle)`, share an `Arc<Mutex<ScenarioState>>` (login/spawn/disconnect bookkeeping, per Context's azalea-integration pattern) into the handler closure, spawn the whole `.start(Account::offline(&config.username), format!("{}:{}", config.host, config.port))` call inside `tokio::time::timeout(config.login_timeout + config.idle_duration + <a small fixed grace, e.g. 5s>, ...)`, and after `Event::Spawn` is observed, drive an explicit `tokio::time::sleep(config.idle_duration)` (not merely relying on the outer timeout, so a clean, on-time client-initiated disconnect can follow), watching a `tokio::sync::Notify` (or oneshot) the handler signals on `Event::Disconnect` throughout, racing it against the sleep via `tokio::select!`. Map every combination of (was `Event::Login` ever seen, was `Event::Spawn` ever seen, did the sleep complete vs. did a disconnect signal win the race) onto the exact `ScenarioOutcome`/`ScenarioError` shapes Deliverables specifies. Observable: `idle_stability_self_tests.rs` passes.
8. **`xtask/src/m1_report.rs`.** Implement `Mode::idle_duration`, `M1ReportResult`, and `run` per Deliverables' doc comment — reusing `tier_result::write_to`/`exit_code_for` unmodified for the flattened-JSON write. Observable: `m1_report_cli.rs` cases 1–3 pass.
9. **`xtask/src/path_guard.rs`.** Apply the two-row replacement from Deliverables. Observable: `m1_report_cli.rs` case 4 passes.
10. **`xtask/src/main.rs`, `xtask/Cargo.toml`.** Add the `M1Report` variant, its `match` arm, the two new path dependencies, and `tokio.workspace = true`. Observable: `cargo build -p xtask` succeeds; `cargo run -p xtask -- m1-report --help` prints usage.
11. **`.github/workflows/ci.yml`.** Append the `m1-acceptance` job and the `workflow_dispatch.inputs.m1_report_mode` block exactly as specified; every other job's YAML is untouched. Observable (once pushed): the new job is visible in the workflow's job graph and does not run on `push`/`pull_request` events (`if:` condition per Deliverables).
12. **`docs/MANUAL-VERIFICATION-M1.md`.** Create exactly as specified.
13. **Run the full acceptance suite.** `cargo nextest run -p rc-test-harness -p rc-paritybot -p xtask` — every test named in Acceptance tests now passes (was red against `todo!()` stubs after the test-authoring changeset). Commit this blueprint's governance changeset with `Changeset-Type: governance` (it touches `xtask/**`, per Constraints).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding**, per Acceptance tests' own stated boundary above; the governance changeset must not edit any of the listed test files or weaken/delete/`#[ignore]` any case in them.

(b) **This blueprint's implementation changeset is a governance changeset**, not an `implementation` one — it touches `xtask/**` (M0-B08's own protected-path rule #7) by necessity (`m1_report.rs`, the `path_guard.rs` correction, `main.rs`). Every commit carries `Changeset-Type: governance`, matching M0-B08's own precedent for blueprints whose job is verification tooling.

(c) **No new external dependencies beyond the pinned set**, with exactly one addition to `[workspace.dependencies]`: `azalea` (git, `rev`-pinned — TEST-D8's own reviewed exception to TEST-D35's "no unpinned git dependencies" rule; the exact `rev` is resolved and recorded at implementation time, never left as a floating branch reference in the committed `Cargo.toml`). No other crate is added anywhere in this blueprint's deliverables — `serde_json`/`thiserror`/`tokio`/`serde` are all already workspace-pinned; `status_probe`'s CLI parsing is hand-rolled specifically to avoid pulling `clap` into a non-`xtask` crate (`clap` remains xtask-only per `12-workspace-structure.md`'s own dependency-versions note).

(d) **No Mojang or third-party reimplementation code.** The fake-server cheat sheet's field layouts are sourced from `docs/research/mc-26.2/02-network-protocol.md` (ASSET-D18(f), the legally-consulted 26.2 reference, described in this project's own words, no verbatim decompiled text reproduced) and from minecraft.wiki's public protocol documentation. Consulting `azalea-protocol`'s own current source for the `ClientboundLogin`/`CommonPlayerSpawnInfo` field list and for verifying the two flagged sub-field wire encodings is **not** a violation of ASSET-D30's third-party-reimplementation firewall: azalea is a *client* library (this project already takes it as a normal, TEST-D8-approved dependency), not another Minecraft *server* reimplementation — ASSET-D30's firewall governs the latter category exclusively (Pumpkin and similar are its named examples), and reading a dependency's own source to use its own API correctly is ordinary engineering practice, not a reimplementation-firewall concern.

(e) **`rc-test-harness` stays synchronous, dependency-minimal, and `tokio`-free.** Every deliverable in that crate uses plain `std::net`/`std::process`/`std::thread` I/O — no async runtime. `rc-paritybot` is the only new crate in this blueprint that depends on `tokio`/`azalea`.

(f) **No CI job or automated test in this blueprint's deliverables ever sets `offline: false` or constructs `azalea::Account::microsoft(...)`.** The single interactive example named in `docs/MANUAL-VERIFICATION-M1.md` is deliberately never wired into `cargo nextest run`'s default test set (an `examples/` target, run only by explicit human invocation) and is not part of this blueprint's own Acceptance tests.

(g) **No `unsafe` code.** Nothing in this blueprint's deliverables requires it.

(h) **Scope boundary.** This blueprint does not implement TEST-D9/TEST-D10's packet-stream/world-state comparator, does not call `xtask setup-oracle`, does not touch `crates/testing/rc-golden-data`, `rc-gametest`, or `rc-chaos` (still reserved paths), and does not modify `M0-B08`'s `tier_result.rs` struct shapes (`m1_report.rs` wraps `TierResult`, never edits it).

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-test-harness -p rc-paritybot -p xtask --all-features
cargo nextest run -p rc-test-harness -p rc-paritybot -p xtask
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- path-guard
cargo run -p xtask -- m1-report --help
```

Expected: every command exits 0. `cargo test --doc -p rc-test-harness -p rc-paritybot` also exits 0. CI's `gates`/`guardrails` jobs green on both OS legs on a clean checkout (TEST-D50) is this blueprint's own authoritative Done signal — a local pass alone does not close it. The new `m1-acceptance` job's own first green run (nightly `smoke`, then a manually-triggered `full`) is a *separate*, later signal — the one that closes `11-roadmap-milestones.md`'s M1 Acceptance Criterion 1 itself, once M1-B02/M1-B04/M1-B05 have also landed — not part of this blueprint's own Done state (Context, "What this blueprint's own CI gate proves").
