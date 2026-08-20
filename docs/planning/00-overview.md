# Overview

## Purpose

This is the entry point to Rusty Clanker's planning documentation, for humans and for AI models deriving implementation blueprints from it. It condenses the vision, states the current phase, maps every planning document, lists the most load-bearing decisions by ID, recaps the documentation conventions, and defines project-specific terms. It restates nothing that a domain document already owns — everywhere else, follow the reference.

## Vision

1. Rusty Clanker is a from-scratch, memory-safe Rust reimplementation of a Java Edition Minecraft-compatible server (Phase 1) and, later, a native client (Phase 2) — the pinned version's officially-distributed, unobfuscated jars may be consulted as a local reference (ASSET-D18/D19), but Mojang expression is never copied into any project artifact.
2. The server core is a standalone `bevy_ecs`; the world is partitioned into independently-ticking **regions**, not one giant simulation loop.
3. Each region's work splits into five fixed domain groups (block/redstone, AI+physics, lighting, chunk serialization, network codec) with statically declared, conflict-checked component access — multiple threads can work the same chunk concurrently, a strict improvement over prior region-threading designs.
4. Regions run on a custom elastic, work-stealing worker pool that grows/shrinks with load and batches quiet regions onto few workers while hot regions scale out — the "dynamic worker pools" of the project's name.
5. All cross-partition communication — region-to-region today, node-to-node in cluster mode — goes through one interface-first, location-transparent message-passing substrate: a `Transport` trait with exactly two implementations.
6. That same substrate is what makes **dual deployment modes** possible: monolithic (single process) and cluster (multiple node processes computing one shared world — server meshing) are switched purely by which `Transport` is wired in, with no domain-logic change between them.
7. Cluster mode adds a connection-terminating proxy and a seamless, buffered handoff protocol, so a player crossing a node boundary sees zero disconnect and no loading screen.
8. Default behavior is bit-identical vanilla parity everywhere, with only a small number of explicitly documented, bounded exceptions — never silent drift.
9. Once the server is proven, a native `wgpu`/`winit` client follows, sharing physics/registry/protocol crates unmodified with the server and reading assets only from the player's own legally-owned local Minecraft installation.
10. Modding is isomorphic: one compiled mod artifact (sandboxed WASM by default, trusted native opt-in) carries shared/server/client logic and loads automatically on whichever side applies.
11. Everything rests on a bring-your-own-assets legal model, source-referenced but never source-copied (ASSET-D18/D19): the pinned version's official unobfuscated jars may be consulted as a local reference, Mojang expression is never copied into any project artifact, and no Mojang-authored asset is ever committed or shipped — only the player's or operator's own legally-obtained game data, read at runtime.
12. The end-to-end goal: a legally and technically sound, high-performance, horizontally-scalable, moddable Minecraft-compatible platform — server first, client second.

## Phase Model

- **Phase 1 — Server.** The dedicated server: engine core, protocol, world/persistence, worldgen, game mechanics, modding API, cluster mode. Sequenced as milestones `M0`–`M8` (`11-roadmap-milestones.md`).
- **Phase 2 — Client.** The native `wgpu` client, isomorphic-mod client-side loop, and singleplayer (embedded server). Sequenced as milestones `M9`–`M10`. Does not begin in earnest until Phase 1 reaches `M8` (`PLAN-D1`).
- **Current phase: PLANNING.** The `docs/planning/` document series (`01`–`13`, plus this file) is the entirety of the project's output to date — no `cargo init`, no source crate, no code exists yet (`PLAN-D6`).
- **Next phase: blueprint derivation**, performed per milestone, not once for the whole project. A milestone's blueprint (one file under the reserved `blueprints/` directory, `WS-D8`) is derived from the planning docs only once that milestone becomes active, starting with `M0`'s blueprint drawn from `01`, `02`, `12`, and `11` (`PLAN-D6`).

## Document Map

| File | Owns | Summary |
|---|---|---|
| `01-server-architecture.md` | ECS core, threading model, tick pipeline, message-passing substrate | Fixes the `bevy_ecs`-based region-partitioned engine core: RC-Executor/RC-WorkerPool, the 11-stage tick pipeline, and the `Transport`-trait substrate every cross-partition interaction — and later cluster mode — builds on. |
| `02-protocol-networking.md` | Java Edition wire protocol, packet codec, connection lifecycle | Pins the tracked Minecraft version, the packet field-layout/codegen pipeline, the connection state machine, encryption/online-mode, and the network↔ECS boundary. |
| `03-world-chunks-persistence.md` | Chunk data model, light engine, on-disk/cluster persistence | Defines the ECS chunk-component decomposition, paletted block/biome storage, the push-model light engine, Anvil `.mca` persistence, and the cluster-mode shared-storage abstraction. |
| `04-worldgen-parity.md` | World generation | Implements vanilla worldgen as a data-driven interpreter over extracted vanilla JSON, with a bit-identical parity acceptance criterion and a two-tier verification strategy. |
| `05-game-mechanics.md` | Server-side gameplay parity | Maps every vanilla gameplay subsystem — redstone, fluids, entities/AI, physics, combat, items, data-driven content, player lifecycle, commands — onto `01`'s domain groups and tick stages. |
| `06-modding-api.md` | Isomorphic modding system | Defines the two-tier (sandboxed WASM / trusted native) mod delivery model, ECS hook integration, cluster-transparent messaging, API surface, versioning, and security model. |
| `07-client-architecture.md` | Phase 2 native client | Defines the `wgpu`/`winit` render stack, vanilla-parity meshing/resource pipelines, netcode/prediction, and the shared-crate boundary with the server. |
| `08-assets-auth-legal.md` | Authentication, asset acquisition, legal/source policy | Defines the client's Microsoft/Xbox auth chain, the local-only asset-acquisition rule, the Mojang-data commit/custody boundary, and the binding source and branding/EULA policy. |
| `09-testing-quality.md` | Verification methodology | Defines the layered parity-verification stack (golden data, vanilla-differential testing, worldgen corpus, in-world structure tests), determinism/cluster testing, fuzzing, CI tiers, and performance SLOs. |
| `10-prior-art.md` | Competitive/prior-art survey | Surveys existing Minecraft-server/client reimplementations and records what this project adopts as validated architecture versus deliberately avoids. |
| `11-roadmap-milestones.md` | Delivery sequencing | Sequences the whole project into milestones `M0`–`M10` with measurable acceptance criteria, a risk register, and the current-phase/next-phase statement. |
| `12-workspace-structure.md` | Cargo workspace | Fixes the full crate list, dependency-graph hard rules, toolchain/edition pin, feature-flag strategy, and on-disk repository layout every other document's code compiles into. |
| `13-cluster-architecture.md` | Multi-node cluster mode | Defines `NetworkTransport`, the raft-backed `RegionId → NodeId` directory, the connection-terminating proxy, and the seamless cross-node player-handoff protocol. |
| `14-performance-engineering.md` | Cross-cutting performance engineering | Owns the parity-gated fast-path framework, memory/allocation/SIMD/build-pipeline/OS-tuning policy, and the concrete performance budgets/reference hardware every other document's performance-adjacent decisions point back to. |

## Foundational Decisions

The most load-bearing decisions across all documents, by ID. This is a pointer index, not a summary of record — read the owning document's Decisions table for full text and rationale.

| ID(s) | What it pins |
|---|---|
| `NET-D1`, `NET-D2` | Pinned parity target — Java Edition 26.2, protocol 776 — tracked as a single version, bumped only via a deliberate, reviewed process, never a multi-version compatibility shim. |
| `ARCH-D1`–`ARCH-D3` | ECS core is standalone `bevy_ecs`; tick execution runs on a custom RC-Executor/RC-WorkerPool, never `bevy_ecs`'s own scheduler. |
| `ARCH-D5`, `ARCH-D6` | World partitioned into independently-ticking **regions** built from 16×16-chunk grid cells, merged/split by load with hysteresis. |
| `ARCH-D8` | Five fixed domain groups per region with a statically-computed startup conflict graph — the mechanism letting multiple threads work one chunk concurrently. |
| `ARCH-D12`–`ARCH-D17` | Fixed 11-stage tick pipeline with a per-stage parallelization axis and determinism guarantee; Stage 4 (redstone) is always fully sequential, single-worker. |
| `ARCH-D24`–`ARCH-D30` | The location-transparent message-passing substrate: addressing types, envelope, `Transport` trait, `InProcessTransport`, delivery/ordering guarantees, ECS-facing `RegionMessageBus` API. |
| `ARCH-D26` | `Transport` has exactly two implementations — the sole seam between monolithic and cluster mode; no domain system changes between them. |
| `CLUSTER-D1`, `CLUSTER-D5` | Cluster's partition unit is the Region, unmodified; a raft-committed `RegionId → NodeId` directory is the sole ownership authority. |
| `CLUSTER-D11`, `CLUSTER-D20`–`D23` | Cluster transport is QUIC (`quinn`) + `postcard`; a connection-terminating proxy owns encryption/session-validation and mediates the seamless (≤2-tick, zero-disconnect) handoff protocol — server meshing's concrete mechanism. |
| `GEN-D8` | Worldgen is a data-driven interpreter over extracted vanilla worldgen JSON, not a hand-ported algorithm. |
| `GEN-D1` | Worldgen parity acceptance criterion: bit-identical output, with exactly one documented, bounded exception (`GEN-D20`). |
| `MOD-D1` | Two-tier isomorphic mod delivery: sandboxed WASM (default) or trusted native `cdylib` (opt-in), one artifact per mod for both server and client. |
| `ASSET-D1`, `ASSET-D13`, `ASSET-D16` | Only the client authenticates with Microsoft/Xbox; no Mojang/Microsoft asset is ever fetched or shipped by engine code — client and server both read only locally- or operator-supplied, legally-obtained data at runtime. |
| `NET-D3` | No dependency on other reimplementations' protocol crates; the wire codec is hand-written from public documentation and the allowed reference sources (ASSET-D18). |
| `WS-D2`, `WS-D3` | Closed Cargo workspace crate manifest with CI-enforced dependency-graph hard rules — shared logic never depends on render/network; simulation never depends on network or render. |
| `TEST-D39` | Every implementation blueprint must define a concrete, layered acceptance-test list per feature; a feature is parity-complete only once it passes. |
| `TEST-D46`, `TEST-D50` | CI is the sole authority on task completion — an implementation changeset is mechanically blocked from touching tests, fixtures, or budget/SLO tables, and an agent's self-reported local run is never a substitute for a green, from-clean-checkout CI run. |
| `PERF-D1`, `PERF-D4` | The parity-gated fast-path framework: a startup-only `EngineConfig`-selected trait seam (`PERF-D1`) admitted only behind an observational-equivalence promotion suite (`PERF-D4`) — the mechanism every optimized fast path across the corpus, current and future, must route through. |
| `PLAN-D1`, `PLAN-D3` | Phase 1 precedes Phase 2; the message substrate and region model are foundational from milestone `M0`, cluster mode is a later activation milestone that touches no `M0`–`M6` domain logic. |

## Conventions

- **Current-state-only.** Every document describes what is decided as of this writing — never a changelog, never a narrated history of alternatives considered or superseded.
- **Decision-ID scheme.** Every binding decision carries a stable ID, `<PREFIX>-D<n>`, one prefix per owning document (`ARCH`, `NET`, `WORLD`, `GEN`, `MECH`, `MOD`, `CLIENT`, `ASSET`, `TEST`, `PRIOR`, `PLAN`, `WS`, `CLUSTER`, `PERF`), monotonically numbered and never renumbered or reused. Other documents, and future blueprints, cite a decision by ID rather than restating it.
- **English throughout.** All planning prose, and all future code/comments/commit messages, are written in English.
- **Doc structure template.** Every domain document (`01`–`13`) follows the same shape: **Purpose** (one paragraph) → **Scope** (In scope / Out of scope) → **Decisions** (a table: ID | Decision | Rationale) → supporting diagrams (Mermaid) where a picture clarifies a pipeline or protocol → **Interfaces** (Provides to / Needs from, per related document) → **Open Questions**. This file is the one exception, structured as an index instead.

## Glossary

- **Region** — the unit of world ownership: a contiguous set of chunks backed by one `bevy_ecs::World`, ticked on its own independent 20 TPS clock (`ARCH-D5`).
- **Grid cell** — the fixed 16×16-chunk (256×256-block) building block regions are assembled from and merge/split along (`ARCH-D6`). Not the same "16×16" as a chunk section.
- **Chunk section** — a 16×16×16 block cube, the vertical subdivision of one chunk column (`03`).
- **Partition** — the general term for an owned, independently-ticking unit of world state: a region in monolithic mode, a region-on-a-node in cluster mode.
- **Domain parallelization** — running the five fixed per-region system groups (Block/Redstone, AI+Physics, Lighting, Chunk Serialization, Network Encode/Decode) concurrently against one shared `bevy_ecs::World`, safe because each group's component access is statically declared and conflict-checked at startup (`ARCH-D8`).
- **Message substrate** — the location-transparent addressing + envelope + `Transport` trait stack that all cross-partition communication goes through, with `InProcessTransport` and `NetworkTransport` as its two implementations (`ARCH-D24`–`D30`, `13`).
- **Monolithic mode** — single-process deployment; partitions communicate via the in-memory `InProcessTransport`.
- **Cluster mode** — multi-node deployment (server meshing); partitions communicate via the QUIC-backed `NetworkTransport`, coordinated by a raft-committed `RegionId → NodeId` directory and a connection-terminating proxy.
- **Sector handoff (handoff protocol)** — the proxy-mediated, packet-buffering, zero-disconnect protocol that moves a player's live connection from one node to another as their owning region crosses a node boundary (`CLUSTER-D22`).
- **Border halo** — a region's read-only mirror of a neighboring region's bordering chunk data, used for wide-radius reads across a partition boundary (`ARCH-D11`).
- **Ghost margin** — the intra-region analog of a border halo: each light-engine BSP round's snapshot of a neighboring chunk's edge values, refreshed every round instead of every tick (`WORLD-D9`).
- **Co-location migration** — the rebalancer moving one of two regions so that a sustained hot cross-node border collapses to zero latency by becoming same-node (`CLUSTER-D8`).
- **Parity level / parity exception** — the project's default bit-identical-to-vanilla behavior, and the small set of explicitly documented, bounded deviations from it (e.g. `GEN-D20`, `ARCH-D14`).
- **Isomorphic mod** — one compiled mod artifact carrying shared/server/client logic, loaded automatically on whichever side(s) apply, with no author bookkeeping for "am I on the server" (`06`).
- **Allowed/forbidden sources** — the binding source lists governing how Mojang-compatible behavior may be implemented: the pinned version's officially-distributed decompiled source may be consulted as a local reference, but Mojang expression is never copied verbatim (`ASSET-D18`/`D19`).
- **Blueprint (blueprint derivation)** — the project phase after planning: deriving a concrete implementation plan for one milestone at a time from the current state of these planning documents (`PLAN-D6`).
