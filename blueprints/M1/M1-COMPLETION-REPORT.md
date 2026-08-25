# M1 Completion Report — Protocol Bootstrap: Status & Login

Integration pass over blueprints M1-B01–M1-B06, run against the actually-committed
`main` branch (commit `2a0a923` at the time this report was written). Covers: (1) the
full CI-equivalent gate suite, (2) the three roadmap acceptance criteria
(`11-roadmap-milestones.md`, "M1 — Protocol Bootstrap"), (3) commit-history trailer
discipline over the whole M1 range, (4) consolidated deviations/open problems from every
per-blueprint agent plus this integration pass, and (5) the manual-verification
instructions Acceptance Criterion 3 requires from the project owner.

**Bottom line: M1 is not yet done.** The gate suite is fully green. Criterion 2 (raw TCP
probe) passes. Criterion 1's status/pong half passes; its Login→Configuration→Play→spawn
half progressed substantially during this pass (three real, previously-undiagnosed
protocol bugs fixed) but still fails against a real client on one further, cleanly
diagnosed, unfixed gap — see "Criterion 1" below. Criterion 3 is unautomatable by design
and awaits the project owner's one-time manual run.

## 1. Gate suite (CI-equivalent, run from a clean tree)

| Gate | Command | Result |
|---|---|---|
| Format | `cargo fmt --all -- --check` | clean |
| Lint | `cargo run -p xtask -- lint` (`cargo clippy --workspace --exclude rc-paritybot --all-targets -- -D warnings`) | clean |
| Dependency graph | `cargo run -p xtask -- lint-deps` | **0 forbidden edges across 27 workspace crates** |
| Guardrail tooling's own suite | `cargo nextest run -p xtask` | **95/95 passed** |
| Full workspace tier | `cargo run -p xtask -- test` (nextest --workspace --exclude xtask --exclude rc-paritybot, then the same under `--features monolithic`, then `cargo test --doc`) | **254/254 passed, 1 skipped** (the by-design `#[ignore]`d `manual_real_sessionserver` test) · **39/39 passed** (monolithic-feature leg) · doctests: 0 runnable examples, clean |

All five gates are green from a clean checkout. `cargo build --workspace --exclude
rc-paritybot` and `cargo clippy --workspace --exclude rc-paritybot --all-targets -- -D
warnings` are both clean with zero warnings.

A real, previously-latent bug in the gate suite itself was found and fixed during this
pass: `rc-protocol::proptest_roundtrip::frame_roundtrip_arbitrary_payload_no_compression`
could nondeterministically fail (every prior M1-B0x agent's report independently
rediscovered and flagged this as "pre-existing, out of scope" without fixing it) because
its input range included the empty-payload/`CompressionState::Disabled` combination,
which `try_decode_frame` correctly and unconditionally rejects as `ZeroLengthFrame` by
design (the same exclusion the sibling unit test `frame_roundtrip_empty_payload` already
locks down). Fixed by narrowing the property's input range (test-authoring commit
`cbac642`) rather than changing `frame.rs`'s own behavior — see §6 for a note on a
**different**, incompatible fix for the same bug sitting on an unrelated branch.

## 2. Acceptance criteria — measured results

### Criterion 2 (raw TCP probe) — **PASS**

> "A raw TCP probe (not a Minecraft client) confirms the Status Response JSON carries
> the correct protocol number (776, NET-D1), version name, online/max player count, and
> MOTD."

Run via `cargo run -p xtask -- m1-report --server-bin target/release/rusty-clanker-server.exe --mode smoke`
against a real, freshly built `--release --no-default-features --features monolithic`
binary. `AC1a_status_pong` and `AC2_status_json_fields` both **pass** — the probe
connects, receives a well-formed `Status Response` (protocol 776, version name, player
counts, MOTD) and a matching `Pong`, every run.

### Criterion 1 (Login→Configuration→Play→spawn, 30-minute idle session) — **FAIL, with measured progress**

> "An unmodified vanilla Java Edition 26.2 client completes Handshake→Status→Pong
> against the server, and separately completes Handshake→Login→Configuration→Play,
> spawns into the superflat placeholder world, and stays connected for a continuous
> 30-minute idle session with zero disconnects or timeouts."

The status/pong half is the same probe as Criterion 2 — pass. The Login→Play→spawn half
is driven by M1-B06's `azalea`-based bot driver (`crates/testing/paritybot`) via
`xtask m1-report`'s `AC1b_login_config_play_spawn` case, which subsumes
`AC1c_idle_stability` (the 30-minute session only starts once spawn is observed).

**Before this integration pass**, the harness could not run at all: the nested
`cargo run` that builds and drives the azalea-dependent bot failed unconditionally with
azalea's own `build.rs` panic ("Azalea currently requires nightly Rust"), even though the
correct nightly toolchain was installed and correctly pinned — see §3.1. This masked
every downstream protocol bug entirely; `AC1b`/`AC1c` had never actually been exercised
against a real client before this session.

**After the toolchain fix**, the first real run surfaced two further real bugs (both
already independently diagnosed, but not fixed, in the M1-B06 agent's own report) that
disconnected every real client during Configuration: an overly strict known-pack
exact-match check, and a Login/Configuration `Disconnect` reason field encoded as JSON
text instead of the network-NBT text component protocol 776 actually requires. Both are
fixed (commit `20f71a7`) — see §3.2/§3.3. A third real bug, a missing `online_mode` field
in `LoginPlay` that would have desynced every field after it, was also found and fixed
(§3.4) by reading azalea's own real packet struct directly, before it could manifest.

**With all three fixes applied**, the bot driver now gets substantially further — it
completes Login, receives `LoginPlay` and the registry-sync `RegistryData` packets, and
begins receiving `SetDefaultSpawnPosition`/chunk packets — but still does not reach
`Event::Spawn`. Measured, reproduced result (both `--mode smoke` and `--mode full`, which
fail identically and equally fast, since the failure is at the Login gate, before any
idle-duration timer starts):

```json
{
  "name": "AC1b_login_config_play_spawn",
  "status": "fail",
  "detail": "no Event::Login observed within the 30s login timeout"
}
```

Diagnosed root cause (via a direct, standalone run of `idle_stability_runner` against a
manually started server, full log captured): the client logs
`Couldn't resolve dimension_type DimensionKind { id: 0 }`, then desyncs and fails to
parse every subsequent packet (`set_default_spawn_position`, `player_position`,
`set_border_center`, `level_chunk_with_light` × many — all "failed to fill whole
buffer"). This is **not** a new bug introduced by this pass — it is the same gap the
M1-B06 agent's own `fake_server.rs` test double already had to work around
(`encode_dimension_type_nbt`, `crates/testing/test-harness/src/fake_server.rs`): the real
production `run_configuration` (`crates/server/src/net/configuration_flow.rs`) sends
every `RegistryData` entry with `has_data=false` unconditionally, on the documented
(M1-B04 Context) assumption that a real client already has its own trusted copy of every
entry it wasn't handed data for. That assumption holds for genuinely static registries
but is **false** for `minecraft:dimension_type` specifically — a real client has no
built-in fallback for it and needs the actual NBT `DimensionKindElement` content
(`height`, `min_y`, …) sent over the wire.

**This gap is not fixed in this pass**, deliberately: fixing it requires giving
`RegistryDataEntryOut` (currently hard-wired to `has_data=false`, `configuration.rs`'s own
explicit `#[rc(nbt)]`/`rc-nbt` deferral) the ability to carry and decode real inline NBT
data — genuinely new wire-format capability, not a mechanical bug fix, and this project's
own binding process (test-first changesets, `TEST-D45/D46`) is not something an
integration pass should improvise around under time pressure. `fake_server.rs`'s own
`encode_dimension_type_nbt` is a ready-made, already-proven reference for the exact
minimal NBT shape a fix needs to produce. **Recommended next step:** a small, dedicated
M1-B04-scope follow-up changeset (test-first) that extends `RegistryDataEntryOut` to
carry optional pre-encoded NBT bytes and wires `run_configuration` to send real
`dimension_type` data using that reference shape.

### Criterion 3 (online-mode session validation) — **not automated, by design**

> "Online-mode session validation (NET-D6) succeeds against Mojang's real session server
> for a genuine purchased account in a manual verification pass (this one step cannot be
> fully automated — it depends on a live third-party account)."

Not performed by any agent in any M1 session, correctly — this needs the project owner's
own real Microsoft/Minecraft account. See §5 for the exact procedure.

## 3. Fixes made in this integration pass

Each is its own commit with a `Changeset-Type` trailer; implementation commits never
touched test/fixture files, per the project's binding process.

### 3.1 `RUSTUP_TOOLCHAIN` leak broke the acceptance harness entirely (governance, `2a0a923`)

`xtask m1_report::run_idle_stability_subprocess` spawns a nested `cargo run` with
`crates/testing/paritybot/` as its working directory specifically to pick up that crate's
own pinned-nightly `rust-toolchain.toml` override (needed because `azalea`'s own
`rust-toolchain.toml` requires nightly, incompatible with this workspace's pinned-stable
root toolchain, WS-D4). When `xtask` itself is launched via `cargo run -p xtask` from the
repo root — exactly how both a developer and the real `m1-acceptance` CI job invoke it —
rustup's own proxy resolves and stamps the *root's* stable toolchain into
`RUSTUP_TOOLCHAIN` in xtask's own environment as a caching optimization.
`std::process::Command` inherits the parent environment by default, so that stamp silently
overrode `paritybot`'s own toolchain file, and the nested `cargo run` used the wrong
(stable) toolchain regardless of the correctly-installed and correctly-pinned nightly.
Reproduced live, both ways: removing `RUSTUP_TOOLCHAIN` from the nested command's
environment (`.env_remove("RUSTUP_TOOLCHAIN")`) fixes it; re-adding it manually
reproduces the exact original failure. **Without this fix, the real `m1-acceptance` CI
job (nightly cron / `workflow_dispatch`) would have failed red on its very first ever
run**, for a reason unrelated to any protocol code.

### 3.2 Configuration's known-pack gate disconnected every real client (implementation, `20f71a7`)

`configuration_flow::drive_until_gate`'s `KnownPacks` gate required the client's echoed
`known_packs` to exactly equal the one pack the server offered — M1-B04's own explicit,
named "defensive design" against an assumption it could not verify without a running
client. Driving a real client proved that assumption wrong: a real, fresh client always
echoes an **empty** list (nothing cached locally). Real vanilla's own known-pack response
is purely informational; this server already sends every registry entry with
`has_data=false` unconditionally, so there is nothing to gate on. Fixed by accepting any
response (a malformed body remains a fatal decode error, only the content-based
mismatch check is removed). The companion test that asserted the old (backwards) behavior
was corrected in its own justified test-authoring commit (`a7b96ba`).

### 3.3 Login/Configuration `Disconnect` reason was JSON, protocol 776 wants NBT (implementation, `20f71a7`)

Both `Disconnect` packets' `reason` field was a plain `WireWrite`-`String` (a JSON text
component, VarInt-length-prefixed). A real client's NBT decoder chokes on this — a short
JSON reason's own VarInt length byte gets misread as an invalid raw NBT tag id, so the
client drops the connection on a decode error instead of ever displaying the intended
disconnect reason. Added `rc_protocol::wire::NbtTextComponent`, a minimal, purpose-built
encoder/decoder for exactly this one field shape (`{"text": "…"}`'s NBT equivalent — not
a general NBT codec, since none exists yet), and switched both `LoginDisconnect.reason`
and Configuration's hand-encoded `Disconnect` payload to it.

### 3.4 `LoginPlay` was missing `online_mode` (implementation, `20f71a7`)

Found by reading `azalea`'s own real `ClientboundLogin` struct directly: `online_mode:
bool` sits between the `CommonPlayerSpawnInfo`-equivalent fields (ending at `sea_level`)
and `enforces_secure_chat`. Without it every field from `enforces_secure_chat` onward
decoded one field short on a real client. Added; hardcoded to `false` for now
(`play::connection::enter_play` has no route to the real `ServerLoginConfig::online_mode`
flag an earlier connection stage already resolved — threading it through is left to
whichever later blueprint wires real Play-state session plumbing, and `false` matches
every automated test and manual-verification path this milestone actually exercises).

## 4. Consolidated deviations (all M1-B01–M1-B06 agents, plus this pass)

Every per-blueprint agent recorded its own deviations in detail in its own session
report; nothing below duplicates that level of detail, only the load-bearing ones with
cross-blueprint consequences:

- **flate2's `zlib-ng` feature was dropped** workspace-wide (M1-B01) — the native backend
  crashed the build sandbox with a `STATUS_ACCESS_VIOLATION`; the pure-Rust `miniz_oxide`
  backend is used instead. Re-adopting a native backend for throughput is explicitly left
  to doc 14's own parity-gated fast-path seam.
- **`azalea` requires a separate, pinned nightly toolchain** (`crates/testing/paritybot/
  rust-toolchain.toml`, `nightly-2026-07-25`) that only applies to that one nested
  subprocess invocation (§3.1) — `rc-paritybot` is excluded from every workspace-wide
  `xtask` gate (`lint`, `test`) and instead lints/tests inside the dedicated
  `m1-acceptance` CI job.
  root `Cargo.toml` needed `exclude = ["crates/testing"]` alongside the new member
  entries for this to resolve at all (Cargo's `"crates/*"` glob otherwise also matches
  the new bare container directory).
- **`RegistryDataEntryOut` has no real NBT support** (M1-B04's own named scope boundary,
  `#[rc(nbt)]` deferred) — this is the root cause behind §2's Criterion 1 gap, now
  concretely proven (not just theorized) to block real-client spawning.
- **Packet ids in `play/packets.rs`** are M1-B05's own live-web-fetch best effort against
  minecraft.wiki, never reconciled against a real, locally-generated `reports/packets.json`
  for protocol 776 (`cargo xtask fetch-data 26.2` needs a legally obtained server.jar,
  unavailable in any automated session so far). Nothing else observed in this pass
  depended on any specific numeric id being wrong, but this is still open.
- **`docs/planning/12-workspace-structure.md`** carries a pre-existing, still-unresolved
  `uuid` version mismatch (`1.24.0` in the doc vs. the actually-pinned `1.25.0`) —
  cosmetic, flagged by M1-B04, not touched by any agent since (out of each one's own file
  scope).

## 5. Open problems (consolidated, most important first)

1. **Criterion 1's dimension_type gap (§2) — the one thing actually blocking M1.** Needs
   a dedicated, test-first M1-B04-scope follow-up giving `RegistryDataEntryOut` real
   inline-NBT-data support, using `fake_server.rs`'s own `encode_dimension_type_nbt` as
   the reference shape. Until this lands, no real vanilla client can spawn into the
   world, regardless of anything else in this report.
2. **Criterion 3's manual step has never been run** (§6, procedure below) — needs the
   project owner's own account, once, after item 1 is fixed and a `full`-mode
   `m1-acceptance` CI run is green.
3. **The real `m1-acceptance` CI job has never run in real GitHub Actions** — only
   locally (Windows only, this session and every prior M1-B0x session). The
   `ubuntu-24.04` leg is completely unverified; nothing in the touched code is
   OS-specific, but this is unverified rather than proven.
4. **Play-state packet ids are unreconciled against a real `packets.json`** (§4) — a
   one-line-per-packet fix once a real, locally-generated report is available.
5. **A pre-existing, unrelated flake**: `xtask/tests/setup_oracle_consent.rs`'s
   `consent_true_via_env_var` races on a shared process-global env var under `cargo
   test`'s default in-process thread parallelism — passes under `cargo nextest run`
   (what CI actually uses) and was not touched by this pass.
6. **The fuzz crate's actual nightly build was never run** (`crates/protocol/fuzz/`,
   M1-B01's own explicitly-deferred manual step) — no nightly + `cargo-fuzz` CLI
   available in any automated session so far.

## 6. A note on a diverging fix for the same frame.rs bug

A separate, concurrent worktree on this machine (`.claude/worktrees/recursing-boyd-10f2f5`,
branch `claude/amazing-hofstadter-095833`, commits `3b6aa67`/`8cbf148`, based on `a705daa`
— **not on `main`**) independently found and fixed the same
`frame_roundtrip_arbitrary_payload_no_compression` bug this report's §1 describes, but
with a **different, incompatible resolution**: it changes `encode_frame`'s own behavior
to return a new `FrameError::EmptyPayload` up front for an empty payload under
`CompressionState::Disabled`, rather than narrowing the property test's input range as
this session did. Both are individually correct and internally consistent, but they are
mutually exclusive — the `main` branch's own fix (this report's, `cbac642`) does not
touch `frame.rs`, so no merge conflict will surface automatically; it needs a **human
decision** about which resolution to keep before that other branch is ever merged, not a
mechanical rebase.

## 7. Commit-history trailer discipline (M1 range: `b045a02`..`2a0a923`, 20 commits)

Every one of the 20 commits in the M1 range (16 from the six per-blueprint agents, plus
this session's own 4: `cbac642`, `20f71a7`, `a7b96ba`, `2a0a923`) carries a well-formed
`Changeset-Type:` trailer (`test-authoring` / `implementation` / `governance`) and a
`Co-Authored-By:` trailer, and each type's file-scope discipline holds — implementation
commits touch only non-test/non-`xtask` source, test-authoring commits touch only
test/fixture files (plus the two `xtask/src` additions M1-B04 and this session each
separately justified as bundled-into-test-authoring or filed as `governance`, matching
M0-B08's own precedent), governance commits touch `xtask/**`/CI/docs.

**Verified mechanically**, not just by inspection: each of the 20 commits' own diff
against its own immediate parent was checked with both `xtask path-guard --base <parent>`
and `xtask lint-tests --base <parent>` in an isolated worktree (so the check reads that
commit's own tree, not the final `main` tip) — **19 of 20 pass cleanly**.

**One anomaly, found and not rewritten (per this task's own instruction):** commit
`7eea682` ("M1-B03: acceptance tests + API stubs for encryption/auth handshake") adds
`crates/auth/tests/manual_real_sessionserver.rs` with
`#[ignore = "requires a real Mojang session and network access — see this blueprint's
Manual verification procedure"]` — a reason string with no `#<digits>` or `issues/<digits>`
substring, which `xtask lint-tests`'s `check_unlinked_ignore` rule requires and correctly
flags as a violation when this commit's own diff is checked against its own parent
(`e358837`). This `#[ignore]` is permanent and deliberate by design (the test needs a
real Mojang account and can never run unattended), not a forgotten TODO — but the lint
rule cannot distinguish that from a genuine gap, and the M1-B03 agent's own session
report does not show `lint-tests` having actually been run (only `fmt-check`/`lint`/
`lint-deps` are listed). **Consequence:** if the orchestrator pushes each of these 20
commits individually — this repository's own established one-push-per-commit pattern —
CI's guardrails job will go red specifically at `7eea682`'s own push, independent of
everything else in this report being green. It does **not** reproduce for a single
batched push of the whole range, since `check_unlinked_ignore` only inspects lines
actually added in the diff being checked, and no fix commit exists yet to correct the
line inside that historical commit itself.

**Not fixed by this integration pass**: the honest remediation is a small follow-up
commit that edits the `#[ignore]` reason to add a qualifying tracking reference (an issue
number or URL) — but doing that meaningfully requires actually opening a tracking issue
on the project's public GitHub repository, which is a "publish public content" action
this pass does not take without the project owner's explicit go-ahead. The project owner
should either open a tracking issue and land a one-line follow-up commit referencing it,
or accept the single red guardrails run on that one historical push as a known,
understood, non-blocking anomaly.

## 8. Manual-step instructions for the project owner — Acceptance Criterion 3

Full procedure already lives at `docs/MANUAL-VERIFICATION-M1.md` (M1-B06); restated here
with more concrete "what to click / how long" detail, per this task's own instruction.
**Perform this only after** item 1 in §5 (the dimension_type gap) is fixed and a
`full`-mode `m1-acceptance` CI run is green — there is no point spending a real account's
login attempt against a server build that cannot yet let any client spawn.

1. **Build and start the server in online mode** (the default — do **not** pass
   `--offline`): `cargo run --release -p rusty-clanker-server -- --bind 0.0.0.0:25565`
   (or your own reachable bind address). Confirm the startup log line shows
   `offline=false`.
2. **Choose one of two ways to connect**, both roughly 2–5 minutes end to end:
   - **(a) Real vanilla client (recommended — this is what the criterion actually
     asks for).** Open the official Minecraft Launcher, sign in with a genuine,
     purchased Microsoft/Minecraft account (the same account you always use), select
     Java Edition **26.2**, click **Play**. Once the game window loads to the main
     menu, click **Multiplayer → Add Server**, enter the server's address, click
     **Done**, then double-click the new server entry to connect.
   - **(b) `rc-paritybot`'s interactive example**, if you don't want to launch a full
     game client: from the repo root, `cargo run -p rc-paritybot --example
     manual_online_check -- <host> <port> <your-email>`. This opens a real Microsoft
     device-code flow directly in your terminal — it will print a short code and a URL
     (`https://microsoft.com/link` or similar); open that URL in any browser, enter the
     code, and approve the sign-in with your own account when prompted. The example
     then attempts the connection itself.
3. **What "pass" looks like:** the world loads and you can see/move around the
   superflat placeholder world — no `Failed to verify username!` / `unverified_username`
   / `authservers_down` disconnect screen. This is the direct, positive proof that
   `rusty-clanker-server`'s real `hasJoined` call against Mojang's live session server
   succeeded for your genuine account.
4. **What "fail" looks like**, and what it means: an immediate disconnect with a
   username-verification or auth-server-down message means the `hasJoined` call itself
   failed — check the server's own log output at the moment of connection for the actual
   HTTP status/error `rc-auth`'s `MojangSessionService` received, and treat this as a
   real NET-D6 bug, not a retry-and-ignore.
5. **Record the result**: date, the Minecraft username used (never credentials or
   tokens — this project's own binding rule), and the exact commit hash of the
   `rusty-clanker-server` build tested, wherever this project tracks milestone sign-off.
   Never automate this procedure and never let any script store or transmit account
   credentials.

## 9. What "M1 done" still requires

In order: (1) the dimension_type/registry-data-sync fix (§5.1, its own test-first
changeset); (2) a green, real `full`-mode `m1-acceptance` GitHub Actions run on both
`ubuntu-24.04` and `windows-2025` (§5.3); (3) the project owner's own one-time manual
Criterion 3 run (§8); (4) a decision on §6's diverging `frame.rs` fix before that other
branch is ever merged. Per `CLAUDE.md`'s hard gate, M1 implementation work stops at that
point — M2 does not start without the user's explicit go-ahead.
