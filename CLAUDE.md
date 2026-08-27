# Rusty Clanker — Project Instructions

Rusty Clanker is a from-scratch Rust reimplementation of the Minecraft: Java Edition server (Phase 1) and a native client (Phase 2), wire-compatible with the vanilla protocol. It targets vanilla gameplay parity on a fully multithreaded, cluster-capable architecture and ships an isomorphic modding API.

## Current phase

**IMPLEMENTATION — milestone M3** (M0 complete 2026-08-24; M1 complete + real-client-verified 2026-08-26; M2 complete + real-client-verified 2026-08-27; M3 started on user go). All milestones M0–M11 are blueprint-complete; code is written strictly from the audited blueprints in `blueprints/` (per-blueprint test-first changesets per TEST-D45/D46, `Changeset-Type` trailer convention per M0-B08). Planning documents in `docs/planning/` remain the source of truth on any conflict. **Hard gate: after each milestone's acceptance criteria are green, STOP — the next milestone starts only on the user's explicit go.** Phase 1 (server) must reach a stable, proven state before Phase 2 (client) implementation begins.

## Binding principles (whole project duration)

- **Best possible result over lowest effort.** Decisions are never made to save work.
- **Docs are current-state only.** Every document describes the current plan as truth. No changelogs, no "options considered" sections, no discarded alternatives, no "we used to think" framing. When something is dropped, it is removed from the documents immediately. A decision may carry a one-sentence rationale.
- **Documentation language: English** (decided 2026-08-20). Code, comments, identifiers, and commit messages: English.
- **Decision IDs.** Every binding decision has a stable ID of the form `<PREFIX>-D<n>` (`ARCH-D5`, `CLUSTER-D22`, …). Other documents and future blueprints reference decisions by ID instead of restating them.
- **Document structure:** `# Title → Purpose → Scope (in/out) → Decisions (with IDs) → design sections → Interfaces (to/from other domains) → Open questions`.
- **Local build hygiene (this machine).** The `target/` tree grows past 25 GB across heavy rebuild cycles (per-test statically-linked debug binaries plus stale hash-suffixed generations cargo never garbage-collects), and side-session worktrees under `.claude/worktrees/` each build their own full `target/` — together they have filled the disk repeatedly. Between big runs (each implementation wave, before full-workspace test runs): delete `target/debug/incremental`, all `*.exe`/`*.pdb` under `target/debug/deps` and `target/release/deps`, and every worktree's `target/` directory. All of it is rebuildable cache; never delete worktree source trees or branches without the user's say-so.
- **Verification is agent-executable and tamper-guarded.** Every test tier runs headless via a single command with machine-readable output — the user is never part of the test loop (one one-time oracle/EULA consent aside). Hard integrity rules: implementation changesets never touch tests, golden fixtures, verification tooling, or budget tables (CI path guard); fixtures are generator-produced and hash-manifested; the vanilla oracle is hash-pinned; a task is done only when the required CI tier is green from a clean checkout — agent-reported local results are advisory. Details: `docs/planning/09-testing-quality.md` (TEST-).

## Legal red lines (non-negotiable)

- **Reference-source policy** (decided 2026-08-20). Decompiled source of the officially-distributed, unobfuscated jars of the pinned version (26.2 ships with readable names; Mojang publishes no obfuscation mappings for it) may be consulted as a research and specification reference, from a legally obtained local copy that never enters the repository. Mojang expression is never copied: no verbatim method bodies in the repository, in docs, or in generated code — behavior, structures, constants, and algorithms are described and reimplemented in our own words. Still forbidden: leaked or unofficially-distributed Mojang source, and any other reimplementation's code (architecture reading stays allowed). Consequence, accepted knowingly: the project no longer claims clean-room provenance (ASSET-D18/D19).
- **Third-party reference firewall (ASSET-D30).** Another reimplementation's source code (e.g. Pumpkin, GPL-3.0) may be examined only by a designated research role, never by implementation or blueprint agents. The research role writes original notes + own-named pseudocode into `docs/research/third-party/`; everyone else consumes only those notes. Primary-source hierarchy: Java SE spec / minecraft.wiki / the ASSET-D18(f) reference first; third-party code only for cross-validation and porting pitfalls.
- **Never commit or ship Mojang-authored content** — no server.jar, no raw data-generator JSON, no textures/sounds/models/structure NBT — in the repository or any release artifact. Only hand-authored specs and code-generated Rust derived from a legally obtained, locally- or CI-fetched copy.
- **Bring your own assets:** the client reads assets at runtime from the user's legally owned local `.minecraft` installation.
- **Authentication:** only the Phase 2 client performs the Microsoft/Xbox login flow; ownership is enforced. The server (and cluster proxy) only calls Mojang's public session-validation endpoint.
- **Dependencies avoid GPL/AGPL/LGPL-family licenses**; no other Minecraft-reimplementation project's code is ever taken as a dependency.
- **Outbound license: AGPL-3.0-only** (ASSET-D31); every external contribution requires a relicensing CLA before merge (ASSET-D32) — never bare DCO/inbound=outbound.
- Branding: no Mojang trademark anywhere; every release-facing surface carries the non-affiliation disclaimer. We distribute an engine, not the game. Legal text in planning docs is engineering policy, not legal advice.

## Load-bearing technical decisions

- **Pinned target: Minecraft Java Edition 26.2** (protocol 776). Single pinned version, bumped only via a deliberate, reviewed process — never a multi-version compatibility layer (NET-D1).
- **ECS: `bevy_ecs` standalone** (no rendering/app/windowing crates) with a **custom work-stealing executor/worker pool** — never bevy's built-in scheduler (ARCH-D1 ff.).
- **Message substrate:** all cross-partition communication — cross-region in-process or cross-node in cluster mode — goes through one `Transport`-trait-based async message substrate; never ad hoc shared state or a second communication mechanism. Within one owned partition, the ECS remains shared-memory.
- **Dual mode, observationally equivalent:** monolithic (default, in-memory lock-free channels, single-container deployment) and cluster (spatial partitioning, QUIC transport, proxy router with seamless zero-disconnect sector handoff). Same engine binary; only the transport implementation differs (13-cluster-architecture.md).
- **Redstone and scheduled block ticking are always fully sequential and single-worker per region** — never parallelized, in any mode.
- **Vanilla parity is bit-identical by default.** Any deviation must be an explicitly documented, bounded, justified exception — never silent or approximate.
- **No cross-partition blocking:** no mod, region, or node interaction may block a tick waiting on another partition; cross-partition effects are fire-and-forget with bounded-latency delivery.
- **Isomorphic mods:** one compiled mod artifact carries shared/server/client parts; the engine loads the applicable sides automatically.
- **Bedrock cross-play (doc 15, CROSS-):** config-activated (`crossplay = false` default — off means zero Bedrock code paths active), own from-scratch translation layer (`rc-bedrock-*` crates) at the connection boundary; Java semantics are always authoritative and the simulation core never learns about Bedrock. Pinned Bedrock protocol tracked independently of the Java pin; RakNet/UDP only; milestone M11 (depends on M0–M7, independent of mod API and client milestones). Geyser/Cloudburst/gophertunnel code only via the ASSET-D30 firewall regardless of their permissive licenses.
- **Performance engineering is owned by doc 14 (PERF-):** parity-gated fast-path framework (every behavior-relevant optimization ships as an alternative backend behind a trait seam, promoted only after an observational-equivalence gate), allocator/arena policy, SIMD dispatch rules, opt-in Cranelift worldgen JIT behind the same gate, PGO/BOLT release pipeline, per-stage tick and memory budgets.

## Document map (`docs/planning/`)

| File | Owns |
|---|---|
| `00-overview.md` | Entry point: vision, doc map, foundational decision register, glossary |
| `01-server-architecture.md` | ECS, threading, tick pipeline, message substrate, monolithic mode (ARCH-) |
| `02-protocol-networking.md` | Vanilla protocol, version pin, connection layer (NET-) |
| `03-world-chunks-persistence.md` | Chunk representation, lighting, NBT, Anvil, storage backends (WORLD-) |
| `04-worldgen-parity.md` | Seed-identical worldgen via vanilla-JSON interpreter (GEN-) |
| `05-game-mechanics.md` | Vanilla gameplay parity, per subsystem (MECH-) |
| `06-modding-api.md` | Isomorphic modding system (MOD-) |
| `07-client-architecture.md` | Phase 2 client: rendering, GUI, audio, prediction (CLIENT-) |
| `08-assets-auth-legal.md` | Auth chain, asset acquisition, reference-source policy (ASSET-) |
| `09-testing-quality.md` | Parity/differential/determinism/cluster testing, CI (TEST-) |
| `10-prior-art.md` | Surveyed projects → adopt/avoid decisions (PRIOR-) |
| `11-roadmap-milestones.md` | Milestones with acceptance criteria, risk register (PLAN-) |
| `12-workspace-structure.md` | Cargo workspace, crate graph, dependency pins (WS-) |
| `13-cluster-architecture.md` | Server meshing: partitioning, ownership, proxy, handoff (CLUSTER-) |
| `14-performance-engineering.md` | Cross-cutting perf: fast-path gate, memory/SIMD/IO tactics, build pipeline, budgets (PERF-) |
| `15-crossplay.md` | Bedrock cross-play: translation layer, Bedrock pin, auth, mapping pipeline, M11 (CROSS-) |

Research corpora (outside `docs/planning/`): `docs/research/mc-26.2/` (subsystem cartography of the ASSET-D18(f) reference) and `docs/research/third-party/` (ASSET-D30 firewall notes).
