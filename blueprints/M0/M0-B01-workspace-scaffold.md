# M0-B01 — Workspace Scaffold & Basic CI Gates

| Field | Content |
|---|---|
| ID | M0-B01 |
| Milestone | M0 — Engine Skeleton & Workspace Bootstrap |
| Prerequisites | — |
| Implements | WS-D1–D12 (crate manifest, dependency-graph rules, toolchain, feature flags, `xtask` command surface, dependency-version pins, repository layout, engine versioning); TEST-D34, TEST-D35 (command text only), TEST-D36 (toolchain policy, restated for consistency with WS-D4) |
| Crates touched | All 22 library crates (`rc-core`, `rc-nbt`, `rc-registries`, `rc-protocol-macros`, `rc-protocol`, `rc-mod-api`, `rc-mod-host`, `rc-messaging`, `rc-transport-inproc`, `rc-transport-net`, `rc-chunk-storage`, `rc-worldgen`, `rc-scheduler`, `rc-mechanics`, `rc-physics`, `rc-entity-macros`, `rc-brigadier`, `rc-auth`, `rc-cluster`, `rc-proxy`, `rc-assets`, `rc-render`) — scaffold only; `rusty-clanker-server`, `rusty-clanker-client` — scaffold only; `xtask` — full implementation |
| Estimated scope | L |

## Goal & Done definition

Stand up the complete Cargo workspace exactly as `12-workspace-structure.md`'s Crate Manifest and Dependency Graph define it — 22 library crates, 2 binary crates, and the `xtask` dev-tooling binary — with every crate's `Cargo.toml` already carrying its target intra-workspace dependency edges (so the WS-D3 dependency-graph rules are meaningfully checkable from the very first commit), plus a working `xtask` exposing the four WS-D9 verbs this blueprint owns (`fmt-check`, `lint`, `lint-deps`, `test`), plus a two-OS GitHub Actions workflow that runs all four on every push and pull request. No crate has real functionality yet: every library crate is an empty shell (a doc comment, nothing else); both binaries print one placeholder line and exit. `xtask` is the only crate with real logic in this blueprint.

Done when:

- [ ] `cargo build --workspace --all-features` succeeds with zero errors and zero warnings on a clean checkout.
- [ ] `cargo run -p xtask -- fmt-check` exits 0 against the freshly-scaffolded tree.
- [ ] `cargo run -p xtask -- lint` exits 0 (zero clippy warnings workspace-wide).
- [ ] `cargo run -p xtask -- lint-deps` exits 0 and reports zero violations against the real, fully-scaffolded workspace graph.
- [ ] `cargo run -p xtask -- test` exits 0 (nextest default run + the `rusty-clanker-server` monolithic-feature run + `cargo test --doc --workspace`, all pass — trivially, since no test content beyond `xtask`'s own unit tests exists yet).
- [ ] `xtask`'s own unit test suite (Acceptance tests section) passes under `cargo nextest run -p xtask`.
- [ ] `.github/workflows/ci.yml` runs the four gates above as a `{ubuntu-24.04, windows-2025}` matrix and is green on both legs.
- [ ] CI tier: this blueprint's own commit must pass Tier 1 as it exists after this blueprint lands — which, at this point in the project, **is** the four gates above (fmt-check, lint, lint-deps, test); no other Tier-1 content (golden-data fixtures, gametest, determinism smoke, proptest, chaos) exists yet, since `09-testing-quality.md`'s TEST-D40–D52 tooling and any content-bearing test suites are out of this blueprint's scope (see Constraints).

## Context (self-contained)

### Workspace identity (WS-D1, WS-D2)

Every library crate is named `rc-<domain>`, living at `crates/<domain>/` with the `rc-` prefix dropped from the directory name. The two shipped binaries are named after the project, not the internal prefix: `rusty-clanker-server` (`crates/server/`) and `rusty-clanker-client` (`crates/client/`). The dev-tooling binary is `xtask` (`xtask/`, workspace root, sibling to `crates/`, **not** under it).

The workspace has **exactly** 22 library crates + 2 binary crates + `xtask` — 25 members total, no more, no fewer. The full manifest (name, path, one-line responsibility) is in [Crate Manifest](#crate-manifest) below, copied from `12-workspace-structure.md` verbatim in substance.

### Dependency-graph hard rules (WS-D3) — restated as a machine-checkable rule set

`xtask lint-deps` is a `cargo metadata`-driven checker (no new dependency needed: it shells out to the `cargo metadata` binary already installed alongside `cargo` itself, and parses the JSON with `serde_json`, already pinned). It enforces exactly four rules, each restated below as data the implementer hard-codes into the checker (exact crate-name lists, not paraphrase):

**Rule 1 — Shared logic presence.** The set `SHARED = [rc-core, rc-nbt, rc-registries, rc-protocol-macros, rc-protocol, rc-mod-api, rc-mod-host, rc-physics]` must be reachable via internal (workspace-member-to-workspace-member) dependency edges from **both** `rusty-clanker-server` and `rusty-clanker-client`, transitively. Violation: any `SHARED` crate absent from either binary's transitive dependency closure.

**Rule 2 — Simulation isolation.** `SIM = [rc-scheduler, rc-mechanics]` must never be transitively reachable from, nor transitively reach, any crate in `NETRENDER = [rc-render, rc-protocol, rc-transport-inproc, rc-transport-net, rc-auth, rc-cluster, rc-proxy]`. Checked in both directions: for every `s` in `SIM`, no `r` in `NETRENDER` may appear in `s`'s transitive closure; for every `r` in `NETRENDER`, no `s` in `SIM` may appear in `r`'s transitive closure.

**Rule 3 — Messaging purity.** `rc-messaging`'s complete set of *normal* (non-dev, non-build) dependencies — internal and external together — must be **exactly** `{rc-core, serde, thiserror}`. No more, no fewer. In particular `crossbeam-channel`, `quinn`, `rc-transport-inproc`, and `rc-transport-net` must never appear as a normal dependency of `rc-messaging`.

**Rule 4 — Mod API is a leaf.** `rc-mod-api`'s complete set of normal dependencies must be **exactly** `{rc-core, bevy_ecs}`.

These four rules are the whole of WS-D3 for lint-deps purposes; TEST-D40–D52's machine-readable-tier-output and CI-path-guard requirements are explicitly **not** implemented by this checker (see Constraints — that is M0-B08's scope). `lint-deps` here just needs to exit 0 on success and exit non-zero with a human-readable violation list on failure.

### Toolchain & edition (WS-D4)

`edition = "2024"` workspace-wide (required by `bevy_ecs` 0.19.1, ARCH-D1). A committed `rust-toolchain.toml` pins `channel = "1.97.0"` with `components = ["rustfmt", "clippy", "rust-src"]`. Edition 2024 implies Cargo resolver v3; declared explicitly (`resolver = "3"`) in the root `[workspace]` table for clarity even though implied. `rust-version = "1.95.0"` in `[workspace.package]` records the MSRV floor (`bevy_ecs`'s stated minimum); the toolchain pin itself (`1.97.0`) is what CI and every contributor actually builds with.

> **Resolved discrepancy:** `09-testing-quality.md`'s TEST-D36 independently states a toolchain pin of `1.97.1`. `12-workspace-structure.md`'s WS-D4 — the document that actually owns `rust-toolchain.toml` (WS-D8's repository layout table assigns that file to WS-D4 explicitly) — pins `1.97.0`. This blueprint follows WS-D4's `1.97.0` as authoritative for `rust-toolchain.toml`'s literal content, since WS-D4/WS-D8 are the specific, file-owning decisions; TEST-D36 is 09's general restatement of the same policy and should be reconciled to match on that document's next revision.

### Feature-flag strategy (WS-D5) and this blueprint's resolved ambiguities

Three independent axes plus one crate-local axis, all Cargo features:

**(a) Cluster activation.** `rc-transport-net`, `rc-cluster`, `rc-proxy` are `optional = true` dependencies of `rusty-clanker-server`, unified under feature `cluster`, which is in `rusty-clanker-server`'s `default` feature list. A minimal from-source build passes `--no-default-features --features monolithic`.

> **Resolved:** `rc-transport-inproc` is a **normal (non-optional)** dependency of `rusty-clanker-server` — present in every build regardless of the `cluster`/`monolithic` feature state. This is required because `InProcessTransport` is used for same-node region-to-region messaging even in cluster mode (WS-D5(a): "Runtime selection between `InProcessTransport`/`NetworkTransport`... remains config-presence-driven... regardless of which features were compiled in"), and because M0's own acceptance criterion 2 (two regions exchanging a message over `InProcessTransport`) must work in a plain default-feature debug build with no special flags. Consequently the `monolithic` feature on `rusty-clanker-server` is declared as an **empty marker feature** (`monolithic = []`) — it exists so `--features monolithic` is a recognized, non-erroring flag name (matching WS-D5(a)'s literal build-invocation text), but it activates nothing on its own since the crate it would "pull in" is already unconditionally present.

**(b) `bevy_ecs` feature surface.** Pinned once in `[workspace.dependencies]` as `default-features = false, features = ["std"]`; every crate needing it inherits via `bevy_ecs.workspace = true`, never redeclaring features.

**(c) Client-side mechanics subset.** `rc-mechanics` exposes default feature `server-systems` (pulls in `rc-scheduler` and tick-system code) and non-default `client-predict` (component *type* definitions and prediction only, no `rc-scheduler`). `rusty-clanker-server` depends on `rc-mechanics` with defaults; `rusty-clanker-client` depends on it with `default-features = false, features = ["client-predict"]`.

> **Resolved:** in `rc-mechanics`'s own `Cargo.toml`, `rc-scheduler`, `rc-chunk-storage`, and `rc-brigadier` are declared `optional = true` and gated behind `server-systems` (`server-systems = ["dep:rc-scheduler", "dep:rc-chunk-storage", "dep:rc-brigadier"]`) — these three are server-tick-only concerns the client-predict subset never needs. `rc-core`, `rc-registries`, `rc-mod-api`, `rc-physics`, `rc-entity-macros` stay unconditional (needed by both variants: component types need registry data, physics prediction, the mod `ComponentDescriptor`, and NBT/metadata derive attributes regardless of which side runs them).

**(d) `rc-chunk-storage`'s `io_uring` feature.** `optional = true`, off by default, gates `io-uring` (pinned `0.7.14`, PERF-D23) as an optional dependency, independent of axes (a)–(c).

### Proxy is a library, not a binary (WS-D6)

`rc-proxy` is a library crate linked into `rusty-clanker-server`, activated at runtime by `role = "proxy"` config — no separate `crates/proxy-bin/` or third binary exists.

### `[workspace.dependencies]` is the single external-version source of truth (WS-D7)

Every member crate inherits external crate versions via `<crate>.workspace = true` — no member crate re-states a version string for any crate also listed in `[workspace.dependencies]`. The complete pinned table is reproduced verbatim in [Root `Cargo.toml`](#root-cargotoml) below. Two crates — `clap` and `xshell` — are **intentionally excluded** from `[workspace.dependencies]` per WS-D7's own text ("xtask-only... kept out of the shipped-binary version set") and are pinned directly in `xtask/Cargo.toml` instead, at the exact versions `12` names: `clap = { version = "4.6.6", features = ["derive"] }`, `xshell = "0.2.7"`.

> **Resolved scope boundary (this blueprint's own decision, not stated verbatim in `12`):** internal RC-crate-to-RC-crate dependencies use plain relative `path = "../<dir>"` entries directly in each consuming crate's `[dependencies]` table, **not** an indirection through `[workspace.dependencies]` — `12`'s own shown root-`Cargo.toml` excerpt lists only externally-versioned crates in `[workspace.dependencies]`, never internal path members, so this blueprint does not invent additional workspace-level entries for them.
>
> **Resolved scope boundary (external-dependency assignment):** this blueprint populates a crate's `[dependencies]` with an **external** (non-workspace-internal) crate **only** where an already-adopted WS-D3 rule requires an exact set to exist from commit 1 — that is exactly `rc-messaging` (Rule 3: `serde`, `thiserror`) and `rc-mod-api` (Rule 4: `bevy_ecs`) — plus `rusty-clanker-server` (needs `tokio`/`toml`/`tracing` to plausibly own the runtime/config/logging bootstrap WS-D2 already assigns it) and `xtask` (needs `clap`/`xshell`/`serde`/`serde_json` to implement its four verbs, which **is** this blueprint's job). Every other crate's external dependencies (`wgpu` for `rc-render`, `simdnbt` for `rc-nbt`, `rsa`/`aes`/`cfb8`/`sha1`/`reqwest`/`rustls` for `rc-auth`, etc.) are added by whichever future blueprint first writes real code in that crate — adding an external dependency to an as-yet-empty crate's manifest today would not be checked by anything `xtask lint-deps` validates (Rules 1–4 above are the only rules that read external dependencies, and they only apply to `rc-messaging`/`rc-mod-api`), so pre-guessing the rest only adds unverifiable content and workspace build time for zero acceptance-criterion benefit. `rc-protocol-macros` and `rc-entity-macros` are proc-macro crates that will eventually need `syn`/`quote`/`proc-macro2` — **none of which `12` has pinned in `[workspace.dependencies]`** — so both crates are scaffolded with `[lib] proc-macro = true` and **zero** dependencies; the blueprint that first writes real macro logic in either crate must add those three crates to `[workspace.dependencies]` via a reviewed planning-document update first (this blueprint must not invent unpinned versions — Constraints, item (b)).

### Repository layout (WS-D8)

```
Rusty Clanker/
├── Cargo.toml                  # workspace manifest (WS-D7)
├── rust-toolchain.toml         # channel = "1.97.0" (WS-D4)
├── .gitignore                  # /target/, /corpus/
├── .cargo/
│   └── config.toml             # [alias] xtask = "run -p xtask --"
├── .github/
│   └── workflows/
│       └── ci.yml              # WS-D11 gates, {ubuntu-24.04, windows-2025} matrix
├── docs/planning/               # already exists, untouched by this blueprint
├── blueprints/                  # already exists, untouched by this blueprint
├── crates/
│   ├── core/  nbt/  registries/  protocol-macros/  protocol/  mod-api/  mod-host/
│   ├── messaging/  transport-inproc/  transport-net/  chunk-storage/  worldgen/
│   ├── scheduler/  physics/  entity-macros/  brigadier/  mechanics/  auth/
│   ├── cluster/  proxy/  assets/  render/  server/  client/
│   └── protocol/{spec/.gitkeep, generated/.gitkeep}   # NET-D9 paths, empty placeholders
├── registries/generated/ under crates/registries/     # → crates/registries/generated/.gitkeep
├── xtask/                       # dev tooling binary (WS-D9), never shipped
└── corpus/                      # NOT created here — git-ignored, populated on demand later (WS-D10)
```

Every one of the 24 `crates/<dir>/` entries gets a `Cargo.toml` and either `src/lib.rs` (library crates) or `src/main.rs` (+ `src/lib.rs` for `rusty-clanker-server` only, per WS-D2's "exposes both a `main.rs` binary target and a `lib.rs` library target").

### `xtask` command surface (WS-D9) — this blueprint's boundary

The full WS-D9 verb surface is `fetch-data <version>`, `codegen` (NET-D9's, unimplemented here), `test`, `bench`, `lint`, `fmt-check`, `lint-deps`, `parity-check <corpus>` — plus `setup-oracle` and CI path-guard/forbidden-pattern checks from `09-testing-quality.md`. **This blueprint implements exactly four**: `fmt-check`, `lint`, `lint-deps`, `test`. The `Command` enum (Deliverables below) is written so a later blueprint adds new variants without touching existing ones. `fetch-data`/`codegen` land with the first protocol-bootstrap blueprint (M1); `bench`/`parity-check` land once there is something to benchmark or parity-check; `setup-oracle` and the path-guard/forbidden-pattern lints are explicitly M0-B08's scope, not implemented, stubbed, or even referenced in `xtask`'s `Command` enum here.

Exact command text this blueprint's four verbs wrap (WS-D9, copied exactly):
- `fmt-check` → `cargo fmt --all -- --check`
- `lint` → `cargo clippy --workspace --all-targets -- -D warnings`
- `test` → `cargo nextest run --workspace`, **plus** (WS-D11's explicit "again with `--no-default-features --features monolithic`" requirement, scoped to the one crate that actually declares those features) `cargo nextest run -p rusty-clanker-server --no-default-features --features monolithic`, **plus** `cargo test --doc --workspace` (TEST-D2: nextest never runs doctests, so the `test` verb must invoke `cargo test --doc` separately to be a complete gate). All three must pass for `xtask test` to exit 0.
- `lint-deps` → the WS-D3 checker specified above (no fixed upstream command text — this blueprint's own algorithm, using `cargo metadata --format-version 1 --all-features` as its one shell-out).

### Testing & benchmarking tooling pin (WS-D10)

`cargo-nextest` **0.9.143**, installed in CI via `cargo install cargo-nextest --locked --version 0.9.143` (not a `Cargo.toml` dependency). `criterion` 0.8.2 is a workspace dev-dependency (unused until a crate adds a `benches/` directory — none does in this blueprint).

> **Resolved discrepancy:** `09-testing-quality.md`'s TEST-D2 independently pins `cargo-nextest` at **0.9.137**, while `12-workspace-structure.md`'s WS-D10 — the file-owning decision for the CI install step this blueprint writes (WS-D8's repository layout table assigns `.github/workflows/ci.yml` to this blueprint, not to `09`) — pins **0.9.143**. This blueprint follows WS-D10's `0.9.143` as authoritative for the literal `cargo install` version string, since WS-D10 is the specific, file-owning decision; TEST-D2 is `09`'s general restatement of the same tooling choice and should be reconciled to match on that document's next revision — the identical resolution pattern this blueprint already applies to the `rust-toolchain.toml` 1.97.0-vs-1.97.1 discrepancy above.

### CI gate policy (WS-D11) and CI matrix (TEST-D34)

Every push and PR runs `fmt-check`, `lint`, `lint-deps`, `test` (with the monolithic-feature re-run described above) — this is the **entire** CI surface this blueprint creates. `TEST-D34`'s matrix: GitHub Actions, OS legs `{ubuntu-24.04, windows-2025}`, no macOS leg, one toolchain leg (the `rust-toolchain.toml` pin, auto-detected by `rustup` — no separate version string duplicated into the workflow file). Nightly/release tiers (parity corpora, throughput/scale SLOs) are **not** created by this blueprint — there is no content for them to run against yet.

`TEST-D35`'s static-gate command text (`cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo-deny`) is **not** wired here: WS-D9's `lint` verb text (no `--all-features`) is what this blueprint's `lint` verb implements, and `cargo-deny` (advisories/licenses/bans/sources) is out of this blueprint's four-gate scope entirely — a later blueprint may add it as a fifth CI job.

### Engine versioning (WS-D12)

`[workspace.package] version = "0.1.0"`, inherited via `version.workspace = true` by every member and both binaries. Independent of the tracked Minecraft protocol version (776 / 26.2) — not touched further by this blueprint.

### Crate Manifest

| Crate | Path | Responsibility (one line) |
|---|---|---|
| `rc-core` | `crates/core/` | Foundational shared types, zero I/O: coordinate math, entity-id types, workspace error/result conventions. Root leaf. |
| `rc-nbt` | `crates/nbt/` | Thin wrapper over `simdnbt` plus an SNBT text reader/writer. |
| `rc-registries` | `crates/registries/` | Canonical block-state/item/biome/entity-type/dimension registry types + generated tables. Also serves as the client's world-model data. |
| `rc-physics` | `crates/physics/` | No-ECS movement/collision/knockback/projectile/vehicle physics; identical compiled code used by server (authoritative) and client (prediction). |
| `rc-entity-macros` | `crates/entity-macros/` | Proc-macro crate: NBT-save and net-metadata derive attributes for entity component fields. |
| `rc-brigadier` | `crates/brigadier/` | Hand-written command-tree parser/dispatcher (Mojang `brigadier` node-graph model). |
| `rc-protocol-macros` | `crates/protocol-macros/` | Proc-macro crate: `#[derive(RcPacket)]` and field-encoding attribute macros. |
| `rc-protocol` | `crates/protocol/` | Wire codec: VarInt/NBT/text-component encode-decode, hand-authored packet spec + generated packet enums. Pure data/codec, no sockets. |
| `rc-mod-api` | `crates/mod-api/` | Isomorphic mod API contract: hook trait signatures, `ComponentDescriptor` builder, manifest schema. Minimal-deps leaf. |
| `rc-mod-host` | `crates/mod-host/` | Engine-side mod loader: dylib loading, ABI boundary, crash isolation, hook-slot registration. |
| `rc-messaging` | `crates/messaging/` | Location-transparent addressing, `Message` envelope, `Transport` trait, `RegionMessage` enum, message bus resource. No transport impl, no network dep. |
| `rc-transport-inproc` | `crates/transport-inproc/` | `InProcessTransport`: `crossbeam-channel`-backed monolithic-mode `Transport` impl + slot-pool allocator. |
| `rc-transport-net` | `crates/transport-net/` | `NetworkTransport`: `quinn`/QUIC + `postcard`-backed cluster-mode `Transport` impl. Gated behind `cluster` feature. |
| `rc-chunk-storage` | `crates/chunk-storage/` | Chunk/section/palette data structures, on-disk region-file format, save scheduling, storage-backend abstraction. |
| `rc-worldgen` | `crates/worldgen/` | Noise pipeline, biome/structure/decoration generation, delivered as Stage-1 structural commands. |
| `rc-scheduler` | `crates/scheduler/` | RC-Executor, RC-WorkerPool, the 11-stage tick pipeline driver, region build/merge/split, Tokio↔RC-WorkerPool boundary types. |
| `rc-mechanics` | `crates/mechanics/` | Concrete domain systems/components. `server-systems` (default) = tick systems; `client-predict` = component types + prediction only. |
| `rc-auth` | `crates/auth/` | Encryption handshake (RSA/AES-CFB8) and Mojang online-mode session validation. |
| `rc-cluster` | `crates/cluster/` | `RegionId -> NodeId` raft-committed directory, rebalancer, membership, failure detection, fencing. Gated behind `cluster` feature. |
| `rc-proxy` | `crates/proxy/` | Proxy-role logic: connection forwarding table, handoff buffering, proxy↔node control channel. A library, not a binary. Gated behind `cluster` feature. |
| `rc-assets` | `crates/assets/` | Locates/parses the player's local `.minecraft` install into engine-usable textures/models/sounds at runtime. Client only. |
| `rc-render` | `crates/render/` | `wgpu`-based rendering pipeline, vanilla-faithful UI + `egui` overlay, `kira` audio. Client only. |
| `rusty-clanker-server` | `crates/server/` | Server composition-root binary + embeddable library target (`run_embedded`). Wires every server-side crate, owns the Tokio runtime. |
| `rusty-clanker-client` | `crates/client/` | Client composition-root binary (Phase 2). Wires client-side crates, owns `winit`/`wgpu` bootstrap and the client ECS/prediction loop. |
| `xtask` | `xtask/` | Dev-only tooling binary. This blueprint: `fmt-check`/`lint`/`lint-deps`/`test`. Never shipped. |

### Internal dependency edges (from `12`'s Dependency Graph, reproduced as data)

| Crate | Internal (`path =`) deps | External normal deps this blueprint declares | Cargo features this blueprint declares |
|---|---|---|---|
| `rc-core` | — | — | — |
| `rc-nbt` | `rc-core` | — | — |
| `rc-registries` | `rc-core`, `rc-nbt` | — | — |
| `rc-physics` | `rc-core` | — | — |
| `rc-entity-macros` | — | — (proc-macro, no deps yet) | `[lib] proc-macro = true` |
| `rc-brigadier` | `rc-core` | — | — |
| `rc-protocol-macros` | — | — (proc-macro, no deps yet) | `[lib] proc-macro = true` |
| `rc-protocol` | `rc-core`, `rc-nbt`, `rc-registries`, `rc-protocol-macros` | — | — |
| `rc-mod-api` | `rc-core` | `bevy_ecs` (Rule 4 exact set) | — |
| `rc-mod-host` | `rc-core`, `rc-mod-api` | — | — |
| `rc-messaging` | `rc-core` | `serde`, `thiserror` (Rule 3 exact set) | — |
| `rc-transport-inproc` | `rc-messaging` | — | — |
| `rc-transport-net` | `rc-messaging` | — | — |
| `rc-chunk-storage` | `rc-core`, `rc-nbt`, `rc-registries` | `io-uring` (optional) | `io_uring = ["dep:io-uring"]`, off by default |
| `rc-worldgen` | `rc-core`, `rc-chunk-storage`, `rc-registries` | — | — |
| `rc-scheduler` | `rc-core`, `rc-messaging`, `rc-mod-host` | — | — |
| `rc-mechanics` | `rc-core`, `rc-registries`, `rc-mod-api`, `rc-physics`, `rc-entity-macros` (unconditional); `rc-scheduler`, `rc-chunk-storage`, `rc-brigadier` (optional, `server-systems`) | — | `default = ["server-systems"]`, `server-systems = ["dep:rc-scheduler","dep:rc-chunk-storage","dep:rc-brigadier"]`, `client-predict = []` |
| `rc-auth` | `rc-core` | — | — |
| `rc-cluster` | `rc-messaging`, `rc-transport-net`, `rc-scheduler` | — | — |
| `rc-proxy` | `rc-cluster`, `rc-transport-net`, `rc-auth`, `rc-protocol` | — | — |
| `rc-assets` | `rc-core`, `rc-registries` | — | — |
| `rc-render` | `rc-core`, `rc-registries`, `rc-assets`, `rc-mod-host` | — | — |
| `rusty-clanker-server` | `rc-core`, `rc-scheduler`, `rc-mechanics`, `rc-chunk-storage`, `rc-worldgen`, `rc-protocol`, `rc-transport-inproc`, `rc-auth`, `rc-mod-host` (unconditional); `rc-cluster`, `rc-transport-net`, `rc-proxy` (optional, `cluster`) | `tokio`, `toml`, `tracing` | `default = ["cluster"]`, `cluster = ["dep:rc-cluster","dep:rc-transport-net","dep:rc-proxy"]`, `monolithic = []` (marker, see above) |
| `rusty-clanker-client` | `rc-core`, `rc-protocol`, `rc-registries`, `rc-nbt`, `rc-assets`, `rc-render`, `rc-physics`, `rc-mod-host`; `rc-mechanics` (`default-features = false, features = ["client-predict"]`) | — | — |
| `xtask` | — | `clap` (`4.6.6`, `derive`), `xshell` (`0.2.7`), `serde` (workspace), `serde_json` (workspace) | — |

Every crate not listed with an explicit `[features]` entry above declares none.

### Empty-shell `lib.rs` / `main.rs` policy

Every one of the 22 library crates' `src/lib.rs` contains **exactly** a module-level doc comment restating that crate's one-line Crate Manifest responsibility from the table above, and nothing else — no `pub` items, no non-default `#![...]` attributes. Example (`crates/core/src/lib.rs`):

```rust
//! `rc-core` — foundational shared types with zero I/O: coordinate math,
//! entity-id types, workspace-wide error/result conventions.
//!
//! M0 scaffold placeholder (M0-B01). Real types land in a later M0 blueprint.
```

The two proc-macro crates (`rc-protocol-macros`, `rc-entity-macros`) use the same pattern but their `lib.rs` may be fully empty (a proc-macro crate library target compiles fine with zero `#[proc_macro_derive]` items).

`rusty-clanker-server`'s `src/lib.rs` is likewise a doc-comment-only placeholder (the real `pub fn run_embedded(...)` per WS-D2 lands with the M0 blueprint that implements `rc-scheduler`'s tick loop). Both binaries' `src/main.rs`:

```rust
fn main() {
    println!("rusty-clanker-server: M0 scaffold placeholder, not yet functional");
}
```

(substitute `rusty-clanker-client` in that crate's copy). Neither binary calls into its own `lib.rs` yet since there is nothing to call.

**There is no public API surface to specify for the 22 library crates or the 2 binaries in this blueprint** — "empty shell" is itself the complete specification for those 24 crates. The only crate with real public API surface is `xtask`, specified in full in Deliverables below.

## Deliverables

### Root `Cargo.toml`

```toml
[workspace]
resolver = "3"
members = ["crates/*", "xtask"]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.95.0"

[workspace.dependencies]
bevy_ecs          = { version = "0.19.1", default-features = false, features = ["std"] }
crossbeam-deque   = "0.8.7"
crossbeam-utils   = "0.8.22"
crossbeam-channel = "0.5.16"
crossbeam-queue   = "0.3.13"
parking_lot       = "0.12.5"
tokio             = { version = "1.53.1", features = ["rt-multi-thread", "net", "time", "sync", "macros"] }
bytes             = "1.12.1"
flate2            = { version = "1.1.9", features = ["zlib-ng"] }
simdnbt           = "0.10.0"
rsa               = "0.9.10"
aes               = "0.9.2"
cfb8              = "0.9.1"
sha1              = "0.11.0"
reqwest           = { version = "0.13.4", default-features = false, features = ["rustls-tls"] }
rustls            = "0.23.43"
quinn             = "0.11.11"
postcard          = "1.1.3"
openraft          = "0.9.25"
redb              = "4.2.0"
serde             = { version = "1.0.229", features = ["derive"] }
serde_json        = "1.0.151"
toml              = "1.1.4"
ron               = "0.12.2"
thiserror         = "2.0.20"
tracing           = "0.1.44"
libloading        = "0.9.0"
wgpu              = "30.0.0"
winit             = "0.30.13"
image             = "0.25.10"
zip               = "8.6.0"
lz4_flex          = "0.14.0"
object_store      = "0.14.1"
rand_xoshiro      = "0.8.1"
wasmtime          = "36.0.13"
wasmtime-wasi     = "36.0.13"
wit-bindgen       = "0.60.0"
stabby            = "72.1.16"
kira              = "0.12.3"
cosmic-text       = "0.19.0"
swash             = "0.2.10"
etagere           = "0.3.0"
egui              = "0.36.1"
egui-wgpu         = "0.36.1"
egui-winit        = "0.36.1"
rayon             = "1.12.0"
keyring           = "4.1.6"
mimalloc          = "0.1.52"
bumpalo           = "3.20.3"
smallvec          = { version = "1.15.2", features = ["union"] }
arrayvec          = "0.7.8"
tinyvec           = "1.12.0"
lasso             = "0.7.3"
core_affinity     = "0.8.3"
pulp              = "0.22.3"
wide              = "1.6.1"
cranelift-codegen = "0.134.3"
cranelift-frontend = "0.134.3"
cranelift-jit     = "0.134.3"
io-uring          = "0.7.14"
rstar             = "0.13.0"
windows           = "0.62.2"
nix               = "0.31.3"

[workspace.dev-dependencies]
criterion         = "0.8.2"

[profile.release]
lto = "fat"
codegen-units = 1
opt-level = 3
panic = "unwind"
debug = "line-tables-only"
strip = false
```

Every version string above is copied byte-for-byte from `12-workspace-structure.md`'s Workspace Dependency Versions table. Do not alter any version while implementing this blueprint. `clap` and `xshell` are deliberately **absent** from this table (WS-D7) — they are pinned only in `xtask/Cargo.toml` directly.

### `rust-toolchain.toml`

```toml
[toolchain]
channel = "1.97.0"
components = ["rustfmt", "clippy", "rust-src"]
```

### `.cargo/config.toml`

```toml
[alias]
xtask = "run -p xtask --"
```

### `.gitignore`

```
/target/
/corpus/
```

### Per-crate `Cargo.toml` template (mechanical — apply to all 22 library crates using the edge table above)

```toml
[package]
name = "rc-<domain>"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
# one line per internal dep, from the edge table:
rc-<other> = { path = "../<other-dir>" }
# plus any external normal dep the edge table lists for this crate, e.g.:
serde = { workspace = true }
```

`publish = false` is added uniformly on every crate (not stated in `12`) to prevent an accidental `cargo publish` — none of these 25 crates is meant to be published to crates.io at this stage.

Worked full examples (every other library crate follows the same shape, substituting its own row from the edge table):

`crates/core/Cargo.toml` (simplest — zero deps):
```toml
[package]
name = "rc-core"
version.workspace = true
edition.workspace = true
publish = false
```

`crates/registries/Cargo.toml` (two internal deps):
```toml
[package]
name = "rc-registries"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-core = { path = "../core" }
rc-nbt = { path = "../nbt" }
```

`crates/protocol-macros/Cargo.toml` (proc-macro, zero deps):
```toml
[package]
name = "rc-protocol-macros"
version.workspace = true
edition.workspace = true
publish = false

[lib]
proc-macro = true
```
(`crates/entity-macros/Cargo.toml` is identical with `name = "rc-entity-macros"`.)

`crates/messaging/Cargo.toml` (Rule 3 exact set):
```toml
[package]
name = "rc-messaging"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-core = { path = "../core" }
serde = { workspace = true }
thiserror = { workspace = true }
```

`crates/mod-api/Cargo.toml` (Rule 4 exact set):
```toml
[package]
name = "rc-mod-api"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-core = { path = "../core" }
bevy_ecs = { workspace = true }
```

`crates/chunk-storage/Cargo.toml` (optional `io_uring` feature):
```toml
[package]
name = "rc-chunk-storage"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-core = { path = "../core" }
rc-nbt = { path = "../nbt" }
rc-registries = { path = "../registries" }
io-uring = { workspace = true, optional = true }

[features]
io_uring = ["dep:io-uring"]
```

`crates/mechanics/Cargo.toml` (the feature-gated example):
```toml
[package]
name = "rc-mechanics"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
rc-core = { path = "../core" }
rc-registries = { path = "../registries" }
rc-mod-api = { path = "../mod-api" }
rc-physics = { path = "../physics" }
rc-entity-macros = { path = "../entity-macros" }
rc-scheduler = { path = "../scheduler", optional = true }
rc-chunk-storage = { path = "../chunk-storage", optional = true }
rc-brigadier = { path = "../brigadier", optional = true }

[features]
default = ["server-systems"]
server-systems = ["dep:rc-scheduler", "dep:rc-chunk-storage", "dep:rc-brigadier"]
client-predict = []
```

### `crates/server/Cargo.toml`

```toml
[package]
name = "rusty-clanker-server"
version.workspace = true
edition.workspace = true
publish = false

[lib]
name = "rusty_clanker_server"
path = "src/lib.rs"

[[bin]]
name = "rusty-clanker-server"
path = "src/main.rs"

[dependencies]
rc-core = { path = "../core" }
rc-scheduler = { path = "../scheduler" }
rc-mechanics = { path = "../mechanics" }
rc-chunk-storage = { path = "../chunk-storage" }
rc-worldgen = { path = "../worldgen" }
rc-protocol = { path = "../protocol" }
rc-transport-inproc = { path = "../transport-inproc" }
rc-auth = { path = "../auth" }
rc-mod-host = { path = "../mod-host" }
tokio = { workspace = true }
toml = { workspace = true }
tracing = { workspace = true }
rc-cluster = { path = "../cluster", optional = true }
rc-transport-net = { path = "../transport-net", optional = true }
rc-proxy = { path = "../proxy", optional = true }

[features]
default = ["cluster"]
cluster = ["dep:rc-cluster", "dep:rc-transport-net", "dep:rc-proxy"]
monolithic = []
```

### `crates/client/Cargo.toml`

```toml
[package]
name = "rusty-clanker-client"
version.workspace = true
edition.workspace = true
publish = false

[[bin]]
name = "rusty-clanker-client"
path = "src/main.rs"

[dependencies]
rc-core = { path = "../core" }
rc-protocol = { path = "../protocol" }
rc-registries = { path = "../registries" }
rc-nbt = { path = "../nbt" }
rc-assets = { path = "../assets" }
rc-render = { path = "../render" }
rc-physics = { path = "../physics" }
rc-mod-host = { path = "../mod-host" }
rc-mechanics = { path = "../mechanics", default-features = false, features = ["client-predict"] }
```

### `xtask/Cargo.toml`

```toml
[package]
name = "xtask"
version.workspace = true
edition.workspace = true
publish = false

[[bin]]
name = "xtask"
path = "src/main.rs"

[dependencies]
clap = { version = "4.6.6", features = ["derive"] }
xshell = "0.2.7"
serde = { workspace = true }
serde_json = { workspace = true }
```

### `xtask` public API surface

```rust
// xtask/src/metadata.rs — parsed `cargo metadata --format-version 1` shape (only the fields
// this blueprint's rule-checker needs; every field name matches cargo's real JSON schema).

/// Top-level `cargo metadata --format-version 1` output.
#[derive(serde::Deserialize)]
pub struct CargoMetadata {
    pub packages: Vec<Package>,
    pub resolve: Resolve,
    pub workspace_members: Vec<String>,
}

#[derive(serde::Deserialize)]
pub struct Package {
    pub id: String,
    pub name: String,
}

#[derive(serde::Deserialize)]
pub struct Resolve {
    pub nodes: Vec<Node>,
}

#[derive(serde::Deserialize)]
pub struct Node {
    pub id: String,
    /// All resolved dependency edges from this node, any kind, as PackageIds.
    pub dependencies: Vec<String>,
    /// Same edges, individually kind-tagged.
    pub deps: Vec<Dep>,
}

#[derive(serde::Deserialize)]
pub struct Dep {
    pub pkg: String,
    pub dep_kinds: Vec<DepKind>,
}

#[derive(serde::Deserialize)]
pub struct DepKind {
    /// `None` = normal dependency; `Some("dev")` / `Some("build")` otherwise.
    pub kind: Option<String>,
}

/// Runs `cargo metadata --format-version 1 --all-features` via `sh` and parses stdout.
/// Returns `Err(<process/parse error message>)` on any failure.
pub fn fetch_metadata(sh: &xshell::Shell) -> Result<CargoMetadata, String>;
```

```rust
// xtask/src/lint_deps.rs

/// One WS-D3 rule violation.
pub struct Violation {
    /// "rule1" | "rule2" | "rule3" | "rule4"
    pub rule: &'static str,
    pub message: String,
}

/// Pure rule-checker: WS-D3 Rules 1-4 against an already-parsed dependency graph.
/// No I/O. This is the function the Acceptance tests exercise directly with
/// synthetic `CargoMetadata` values.
pub fn check_rules(meta: &CargoMetadata) -> Vec<Violation>;

/// CLI entry point for the `lint-deps` verb: fetch + check + print + exit code.
pub fn run() -> std::process::ExitCode;
```

```rust
// xtask/src/fmt_check.rs
pub fn run() -> std::process::ExitCode;

// xtask/src/lint.rs
pub fn run() -> std::process::ExitCode;

// xtask/src/test.rs
pub fn run() -> std::process::ExitCode;
```

```rust
// xtask/src/main.rs

#[derive(clap::Parser)]
#[command(name = "xtask")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Subcommand, Debug, PartialEq)]
pub enum Command {
    /// cargo fmt --all -- --check
    FmtCheck,
    /// cargo clippy --workspace --all-targets -- -D warnings
    Lint,
    /// WS-D3 dependency-graph rule checker
    LintDeps,
    /// nextest (default features) + rusty-clanker-server monolithic + doctests
    Test,
}

fn main() -> std::process::ExitCode;
```

### `.github/workflows/ci.yml`

```yaml
name: CI

on:
  push:
  pull_request:

jobs:
  gates:
    name: gates (${{ matrix.os }})
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

      - name: Cache cargo-nextest binary
        id: nextest-cache
        uses: actions/cache@v4
        with:
          path: ~/.cargo/bin/cargo-nextest*
          key: nextest-${{ matrix.os }}-0.9.143

      - name: Install cargo-nextest (WS-D10 pin)
        if: steps.nextest-cache.outputs.cache-hit != 'true'
        run: cargo install cargo-nextest --locked --version 0.9.143

      - name: fmt-check
        run: cargo run -p xtask -- fmt-check

      - name: lint
        run: cargo run -p xtask -- lint

      - name: lint-deps
        run: cargo run -p xtask -- lint-deps

      - name: test
        run: cargo run -p xtask -- test
```

### Directory placeholders

- `crates/protocol/spec/.gitkeep` (empty file — NET-D9's `crates/protocol/spec/*.ron` files land in a later blueprint)
- `crates/protocol/generated/.gitkeep` (empty file — `crates/protocol/generated/v776/` packet codegen lands with the M1 packet-codegen blueprint; WS-D13 reserves this directory for packet code only)
- `crates/registries/generated/.gitkeep` (empty file — `crates/registries/generated/v776/` registry/block-state codegen lands with M0-B07, WS-D13)

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** this blueprint's test changeset is exactly `xtask/tests/lint_deps_rules.rs` and `xtask/tests/cli_parsing.rs`, plus the minimal `xtask/src/{metadata.rs, lint_deps.rs, fmt_check.rs, lint.rs, test.rs, main.rs}` skeletons with every function body from the public API surface above replaced with `todo!()` (so the test changeset compiles and the tests below can run — they will fail/panic against the `todo!()` stubs, which is the expected red state). The implementation changeset (Implementation steps below) fills in real bodies and creates all 25 scaffolded crates plus `.github/workflows/ci.yml`; it must not modify either test file.

### `xtask/tests/lint_deps_rules.rs`

Constructs `CargoMetadata` values directly as Rust struct literals (no JSON — `check_rules` takes already-parsed data, so tests exercise it directly). A local helper:

```rust
fn node(id: &str, all_deps: &[&str], normal_deps: &[&str]) -> xtask::metadata::Node {
    xtask::metadata::Node {
        id: id.to_string(),
        dependencies: all_deps.iter().map(|s| s.to_string()).collect(),
        deps: normal_deps.iter().map(|s| xtask::metadata::Dep {
            pkg: s.to_string(),
            dep_kinds: vec![xtask::metadata::DepKind { kind: None }],
        }).collect(),
    }
}
fn pkg(id: &str, name: &str) -> xtask::metadata::Package {
    xtask::metadata::Package { id: id.to_string(), name: name.to_string() }
}
```

Test cases (each builds a `CargoMetadata` with `packages`, `resolve.nodes`, `workspace_members` covering only the crates it needs — extra unrelated crates may be omitted):

1. `clean_graph_has_zero_violations` — a graph matching this blueprint's real edge table exactly (all 25 crates, all edges from the table above, `rusty-clanker-server` built with `cluster` active so `rc-cluster`/`rc-transport-net`/`rc-proxy` appear). Assert `check_rules(&meta).is_empty()`.
2. `rule1_flags_missing_shared_crate` — same as case 1 but `rusty-clanker-client`'s node omits the edge to `rc-physics`. Assert exactly one `Violation` with `rule == "rule1"` mentioning `rc-physics` and `rusty-clanker-client`.
3. `rule2_flags_scheduler_reaching_render` — a minimal graph where `rc-scheduler` has an edge (direct, in `dependencies`) to `rc-render`. Assert exactly one `Violation` with `rule == "rule2"`.
4. `rule2_flags_transitive_violation` — `rc-scheduler -> rc-mod-host -> rc-render` (the forbidden crate reachable two hops away, not directly). Assert `rule2` violation still fires (proves the checker does transitive closure, not just direct-edge comparison).
5. `rule2_allows_scheduler_and_mechanics_depending_on_each_other` — `rc-mechanics -> rc-scheduler` edge present (both in `SIM`). Assert **zero** violations from this edge (SIM-to-SIM is allowed; only SIM-to-NETRENDER is forbidden).
6. `rule3_flags_extra_normal_dep` — `rc-messaging`'s node has normal deps `{rc-core, serde, thiserror, crossbeam-channel}` (one extra). Assert exactly one `Violation` with `rule == "rule3"`.
7. `rule3_flags_missing_required_dep` — `rc-messaging`'s normal deps are `{rc-core, serde}` (missing `thiserror`). Assert one `rule3` violation.
8. `rule3_ignores_dev_dependency` — `rc-messaging`'s `deps` list includes an entry for some test-only crate with `dep_kinds = [DepKind { kind: Some("dev".into()) }]` in addition to the exact `{rc-core, serde, thiserror}` normal set. Assert **zero** violations (dev-deps are not counted by Rule 3).
9. `rule4_flags_extra_normal_dep` — `rc-mod-api`'s normal deps are `{rc-core, bevy_ecs, rc-scheduler}`. Assert exactly one `rule4` violation.
10. `multiple_violations_all_reported` — a graph combining cases 3 and 6 simultaneously. Assert `check_rules(&meta).len() == 2`, one of each rule.

### `xtask/tests/cli_parsing.rs`

```rust
use xtask::{Cli, Command};
use clap::Parser;

#[test]
fn parses_fmt_check() {
    let cli = Cli::try_parse_from(["xtask", "fmt-check"]).unwrap();
    assert_eq!(cli.command, Command::FmtCheck);
}

#[test]
fn parses_lint() {
    let cli = Cli::try_parse_from(["xtask", "lint"]).unwrap();
    assert_eq!(cli.command, Command::Lint);
}

#[test]
fn parses_lint_deps() {
    let cli = Cli::try_parse_from(["xtask", "lint-deps"]).unwrap();
    assert_eq!(cli.command, Command::LintDeps);
}

#[test]
fn parses_test() {
    let cli = Cli::try_parse_from(["xtask", "test"]).unwrap();
    assert_eq!(cli.command, Command::Test);
}

#[test]
fn rejects_unknown_verb() {
    assert!(Cli::try_parse_from(["xtask", "not-a-real-verb"]).is_err());
}
```

(Requires `Cli`/`Command` to be `pub` from an `xtask` library target, or `#[path]`-included from `main.rs` into a small `xtask/src/lib.rs` that `main.rs` also uses — implementer's choice of internal wiring, as long as both `main.rs`'s binary behavior and this test's `use xtask::{Cli, Command}` compile.)

### Real-workspace integration check

`xtask/tests/lint_deps_rules.rs` additionally contains:

```rust
#[test]
fn real_workspace_has_zero_forbidden_edges() {
    let sh = xshell::Shell::new().unwrap();
    let meta = xtask::metadata::fetch_metadata(&sh).expect("cargo metadata failed");
    let violations = xtask::lint_deps::check_rules(&meta);
    assert!(violations.is_empty(), "violations: {:?}",
        violations.iter().map(|v| &v.message).collect::<Vec<_>>());
}
```

This test is expected to **fail** (panic on `fetch_metadata`'s `expect`, since `crates/*` does not exist yet) in the test changeset's red state, and to pass once the implementation changeset has scaffolded all 25 crates per the edge table.

## Implementation steps

1. **Root scaffold.** Create `Cargo.toml`, `rust-toolchain.toml`, `.cargo/config.toml`, `.gitignore` exactly as specified in Deliverables. Observable state: `cargo metadata` runs (even with zero `crates/*` members matched by the glob) without error.
2. **`xtask` skeleton (if not already present from the test changeset).** Create `xtask/Cargo.toml`, `xtask/src/main.rs`, `xtask/src/metadata.rs`, `xtask/src/lint_deps.rs`, `xtask/src/fmt_check.rs`, `xtask/src/lint.rs`, `xtask/src/test.rs`, and a small `xtask/src/lib.rs` re-exporting `Cli`, `Command`, and the `metadata`/`lint_deps` modules so `xtask/tests/*.rs` can `use xtask::...`. Observable state: `cargo build -p xtask` compiles (with `todo!()` bodies still in place at this point if starting fresh, or already-real bodies if the test changeset's stubs are being replaced now).
3. **Implement `metadata::fetch_metadata`.** Use `xshell::Shell::new()` then `xshell::cmd!(sh, "cargo metadata --format-version 1 --all-features").read()` to capture stdout, then `serde_json::from_str::<CargoMetadata>(&stdout)`. Map any error (process failure or parse failure) to `Err(String)`. Observable: unit-testable in isolation by running it against the real (eventually-scaffolded) workspace.
4. **Implement `lint_deps::check_rules`.** Build `id_to_name: HashMap<&str, &str>` from `meta.packages`. Build `workspace_ids: HashSet<&str>` from `meta.workspace_members`. Build the internal-only forward graph `HashMap<&str, HashSet<&str>>` from every node whose `id` is a workspace member, using `node.dependencies` filtered to targets also in `workspace_ids`, translated through `id_to_name`. Implement a private `transitive_closure(graph, start) -> HashSet<&str>` via BFS. Then:
   - Rule 1: for `SHARED` crates, assert presence in both `transitive_closure(graph, "rusty-clanker-server")` and `transitive_closure(graph, "rusty-clanker-client")`; push a `Violation { rule: "rule1", .. }` for each miss.
   - Rule 2: for each `s` in `SIM`, compute its closure and check no `r` in `NETRENDER` is present; then symmetric for each `r` in `NETRENDER` against `SIM`. Push `Violation { rule: "rule2", .. }` per hit.
   - Rule 3 / Rule 4: locate the node whose `id` maps to `"rc-messaging"` (resp. `"rc-mod-api"`); collect `node.deps` entries where `dep_kinds.iter().any(|k| k.kind.is_none())`, resolve each `.pkg` through `id_to_name` (never use `Dep`'s absent name field — there isn't one, by design, precisely to sidestep the hyphen/underscore extern-name-normalization ambiguity `cargo metadata` has), collect into a `HashSet<&str>`, compare by set equality against `{"rc-core","serde","thiserror"}` (resp. `{"rc-core","bevy_ecs"}`); push one `Violation { rule: "rule3"/"rule4", .. }` describing the full symmetric difference if unequal.
   - Return the accumulated `Vec<Violation>`.
5. **Implement `lint_deps::run`.** Call `fetch_metadata`, on error print to stderr and return `ExitCode::FAILURE`; on success call `check_rules`; if empty, print `"lint-deps: 0 forbidden edges across {N} workspace crates"` and return `ExitCode::SUCCESS`; else print each violation's `rule`/`message` to stderr and return `ExitCode::FAILURE`.
6. **Implement `fmt_check::run`, `lint::run`.** Each: `Shell::new()`, run the exact WS-D9 command text via `cmd!(sh, "...").run()`, map `Ok(())` to `ExitCode::SUCCESS`, `Err(_)` to `ExitCode::FAILURE` (the underlying command's own stdout/stderr already streams to the terminal via `xshell`'s default behavior — no need to re-print).
7. **Implement `test::run`.** Run, in order: `cargo nextest run --workspace`; if it succeeds, `cargo nextest run -p rusty-clanker-server --no-default-features --features monolithic`; if that succeeds, `cargo test --doc --workspace`. Short-circuit and return `ExitCode::FAILURE` on the first failing step; `ExitCode::SUCCESS` only if all three succeed.
8. **Implement `main.rs`.** `Cli::parse()`, `match` on `cli.command` dispatching to the four `run()` functions, return its `ExitCode`.
9. **Run `xtask`'s own test suite** (`cargo nextest run -p xtask`) against the fixture-based tests (cases 1–10 in `lint_deps_rules.rs` and all of `cli_parsing.rs`) — these must all pass now, using only the fixtures, independent of the rest of the workspace existing.
10. **Scaffold the 22 library crates.** For each, create `crates/<dir>/Cargo.toml` (from the template + edge table row) and `crates/<dir>/src/lib.rs` (empty-shell doc comment, per policy). Observable: `cargo build -p rc-<domain>` succeeds for each, one at a time, in an order that satisfies each crate's own internal deps (e.g. `rc-core` first, then `rc-nbt`, etc. — or simply create all 22 `Cargo.toml`s and `lib.rs`s together and build once, since Cargo resolves the whole graph regardless of file-creation order).
11. **Scaffold the two binaries.** Create `crates/server/Cargo.toml` + `src/lib.rs` + `src/main.rs`, `crates/client/Cargo.toml` + `src/main.rs`, exactly as specified in Deliverables.
12. **Create the three `.gitkeep` directory placeholders.**
13. **Full-workspace build.** `cargo build --workspace --all-features` must now succeed with zero warnings.
14. **Run the real-workspace integration test.** `cargo nextest run -p xtask` — `real_workspace_has_zero_forbidden_edges` must now pass (it was red in step 9's run, since `crates/*` did not exist yet).
15. **Create `.github/workflows/ci.yml`** exactly as specified. Push to a branch and confirm both matrix legs go green (or, if no CI runner is available to the implementer directly, confirm by running the exact four `cargo run -p xtask -- <verb>` commands locally on whichever OS is available, plus `cargo run -p xtask -- test`, and note that CI-is-authority (TEST-D50) still governs final done-ness once pushed).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** `xtask/tests/lint_deps_rules.rs` and `xtask/tests/cli_parsing.rs` are committed first, with every `xtask/src/*.rs` function body they call stubbed `todo!()`. The implementation changeset (steps 3–15 above) fills in real bodies and creates the 25 scaffolded crates plus the CI workflow; it must not edit either test file, and must not weaken, delete, or `#[ignore]` any of the ten `lint_deps_rules.rs` cases or the five `cli_parsing.rs` cases.

(b) **No new external dependencies beyond the pinned set.** Every external crate this blueprint's deliverables use is either in the `[workspace.dependencies]` table reproduced above verbatim, or is `clap`/`xshell` pinned directly in `xtask/Cargo.toml` at the exact versions given (WS-D7's own named exception). Do not add `anyhow`, `cargo_metadata`, `syn`, `quote`, `proc-macro2`, or any other crate not named in this blueprint — `metadata.rs`'s hand-rolled `serde`-based structs exist specifically so `cargo_metadata` is never needed, and the two proc-macro crates are left dependency-free specifically because `syn`/`quote`/`proc-macro2` are not yet pinned anywhere in the planning corpus.

(c) **No Mojang or third-party reimplementation code.** Nothing in this blueprint touches protocol, asset, or worldgen content, so this constraint is inherited rather than actively load-bearing here — but it still applies: no code from a decompiled/leaked Minecraft source, from another open-source MC-server-in-Rust project, or from any other third-party reimplementation is consulted or copied while writing any file this blueprint creates (ASSET-D18/D19/D30).

(d) **Scope boundary — do not implement beyond scaffold.** This blueprint does not implement `rc-core`'s real types, `rc-messaging`'s real envelope/bus, or `rc-scheduler`'s real tick pipeline (ARCH-D1–D9/D12/D18–D23 — a separate M0 blueprint's job); does not implement `xtask fetch-data`/`codegen` (NET-D9 — a separate, later blueprint's job, most likely alongside M1); does not implement `xtask bench`/`parity-check`/`setup-oracle`, the CI path-guard, or the forbidden-pattern lints (TEST-D40–D52, M0-B08's scope); does not wire `cargo-deny` or `cargo-llvm-cov` (TEST-D33/D35, out of this blueprint's four-gate CI scope). Do not add any of these as a shortcut to "look more complete" — every crate this blueprint scaffolds stays an empty shell.

(e) **No `unsafe` code.** Nothing in this blueprint's deliverables — the 24 empty-shell crates or `xtask` itself — uses `unsafe`.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build --workspace --all-features
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
cargo nextest run -p xtask
```

Expected: every command exits 0. `cargo run -p xtask -- lint-deps` additionally prints `lint-deps: 0 forbidden edges across 25 workspace crates` (the count reflects however many workspace-member packages `cargo metadata` reports — 25 if `xtask` itself is counted, since it too is a workspace member subject to Rule 1/2's graph construction even though it appears in neither `SHARED`, `SIM`, nor `NETRENDER` and so never triggers a violation). CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
