# M8-B04 — The Reference Mod: `example_ores`, Its Template Shape, and End-to-End Proof

| Field | Content |
|---|---|
| ID | M8-B04 |
| Milestone | M8 — Mod API Alpha |
| Prerequisites | M8-B01 (`rc-mod-api`'s complete public surface — every type/signature this blueprint builds against is restated below with its exact shape; this blueprint's own edit to `rc-mod-api` is a single, narrow, purely-additive constructor, the same class of permitted continuation M8-B02 already used); M8-B02 (`rc-mod-host`'s `ServerModHost`/`ClientModHost` — real discovery, SHA-256 trust, ABI handshake, `catch_unwind`-based crash isolation; this blueprint loads the reference mod through this exact, already-shipped, already-tested machinery, never a reimplementation of it); M8-B03 (`rc-scheduler`'s `resolve_component_access`/`resolve_hook_order`/`register_mod_system`/`ModSystemShim`/`ModHookInvoke` — this blueprint's own new `rc-scheduler` module is the first real `ModHookInvoke` implementation, additive, built strictly on this blueprint's already-fixed contract); M3-B01 (`rc-mechanics`'s real `BlockBehavior`/`UpdateContext`/`TickPriority`/`Direction`/`BlockBehaviorRegistry` — the vanilla-shaped seam `ModBlockBehavior`/`ModUpdateContext` mirror one-for-one, restated below only to the extent this blueprint's own block behavior must stay faithful to it); M2-B01 (`rc-chunk-storage`'s `PalettedContainer<T>`/`PaletteThresholds`/`BlockStateColumn`'s exact dense-id/bit-width rule — the mechanism this blueprint's own persistence test measures concretely); M2-B04 (`rc-chunk-storage`'s `ChunkNbtCodec`/`BlockStateNames`/`BiomeNames` — block-state identity persists by namespaced name+properties, never by raw numeric id, restated below as the mechanism this blueprint's persistence test proves for a modded block specifically); M4-B01 (`rc-mechanics`'s `ItemStackRecord`/`RegistryEntryId` shape — consulted only to confirm this blueprint's own item content matches the vanilla-shaped item-record convention the real engine will eventually use; this blueprint does not depend on `rc-mechanics` and does not construct a real `ItemStackRecord`); M0-B05 (`rc-scheduler`'s `RcExecutorBuilder`/`RcExecutor`/`RcWorkerPool`/`spawn_region`/`tick_region`/`TickReport` — the exact, already-shipped driver this blueprint's own end-to-end harness calls, unmodified); M6-B07 (consulted only — confirmed, as of M6-B07, that `crates/server/`'s real startup sequence never mentions mods anywhere; this blueprint does not touch `crates/server/` at all, restated in Constraints). |
| Implements | The milestone's own Scope/Acceptance criteria (`11-roadmap-milestones.md`, M8) end to end, using real content instead of the synthetic test doubles M8-B01–M8-B03's own suites already used to prove the underlying mechanism in isolation; MOD-D4/D5 (isomorphic `.rcmod` packaging and per-side load selection, exercised for real for the first time); MOD-D6 (registry insertion — block, item, component, channel — exercised for real); MOD-D8–D12 (declared access, ordering, exclusive-access-adjacent conflict rejection, exercised end to end with a real second mod); MOD-D13 (POD component discipline, restated for `example_ores:ore_charge`); MOD-D17 (no authoritative mod state outside ECS components, restated as this blueprint's own binding design rule for `PulseCrystalBehavior`); MOD-D18 (client extension-point registration, headless-verified, visual deferred to M10 per PLAN-D2); MOD-D20 (custom network channel registration, recorded only); MOD-D25/D32 (crash isolation, exercised end to end through a real dylib's real panic, real disable, real tick-pipeline continuation); MOD-D27 (dev-experience template shape — this blueprint's own reference-mod crate tree *is* the concrete, tested instance of that shape; wiring an actual `cargo generate`-invokable repository remains explicitly deferred, restated in Context); MOD-D31 (dependency-order resolution — this blueprint's two mods carry no `[dependencies]` on each other, so this decision is inherited unexercised beyond what M8-B02 already proved); WORLD-D2 (paletted-container bit-width consequence of installing one small mod, made concrete with real numbers); PLAN-D2 (client-side visual verification explicitly deferred to M10, restated as this blueprint's own binding scope boundary). |
| Crates touched | `mods/example-ores/` (new, three-crate Cargo workspace — `shared/`, `server/`, `client/` — **not** a member of the main Rusty Clanker workspace, see Context); `mods/conflict-probe/` (new, single-crate package, likewise not a workspace member); `rc-mod-api` (`crates/mod-api/`, one additive edit: `block_behavior.rs` gains a single new `pub fn` constructor, no existing signature changed); `rc-scheduler` (`crates/scheduler/`, additive: one new production module bridging `rc-mod-host` dispatch into `rc-scheduler`'s `ModHookInvoke` slot, plus this blueprint's own new test files); `rc-chunk-storage` (`crates/chunk-storage/`, test-only additive: one new integration test file, zero `src/` changes); `.gitignore` (one additive line). |
| Estimated scope | L — a deliberate, cited exception to the ~800-line guideline, the same class M8-B01/B02/B03 already used: this is the blueprint that closes the loop between three independently-shipped mechanisms (`rc-mod-api`'s contract, `rc-mod-host`'s loader, `rc-scheduler`'s conflict graph) using one real, working piece of content, and none of the three seams this closes (the `ModHookInvoke` bridge, `ModUpdateContext`'s missing public constructor, the reference mod's own real behavior) is safely splittable without leaving the others unproven. |

## Goal & Done definition

Ship the reference mod, `example_ores`, as a real, working, isomorphic mod — not a stub — plus the two small, genuinely-new, purely-additive pieces of glue no earlier M8 blueprint built (both explicitly named as gaps by their own Context sections, restated below) that are required to prove it end to end: (1) a public constructor for `rc-mod-api`'s already-shipped, currently-unconstructable-from-outside-the-crate `ModUpdateContext`; (2) `rc-scheduler`'s first real `ModHookInvoke` implementation, bridging a `ModSystemShim`'s dispatch into `rc-mod-host`'s already-crash-isolated `ServerModHost::call_on_tick_hook`. Every other line of this blueprint is content (the mod itself, its manifest, its second-mod conflict fixture) or tests — no other production engine mechanism is touched, and `crates/server/` is not touched at all.

Done when:

- [ ] `cargo build --manifest-path mods/example-ores/Cargo.toml` and `cargo build --manifest-path mods/conflict-probe/Cargo.toml` both succeed with zero warnings, independently of the main Rusty Clanker workspace build.
- [ ] `cargo test --manifest-path mods/example-ores/Cargo.toml` passes in full (the mod's own behavior-unit test suite, against a hand-built headless `ModUpdateContext`/`RegistryBuildContext` harness, no dylib loading involved).
- [ ] `cargo build -p rc-mod-api -p rc-scheduler --all-features` succeeds with zero warnings; every pre-existing `rc-mod-api`/`rc-scheduler` test (M8-B01's full suite, M8-B03's full suite, unmodified) still passes byte-for-byte.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-scheduler -p rc-mod-api -p rc-chunk-storage`.
- [ ] `mod_reference_conflict_graph.rs`'s rejection test asserts a hard `Err(ModOrderingError::Cycle{..})` naming both `example_ores:pulse_survey` and `conflict_probe:counter_tick` by `Identifier` — the milestone's own acceptance criterion 1, closed with two real, separately-compiled, `ServerModHost`-loaded dylibs.
- [ ] `mod_reference_crash_isolation.rs` proves, against the real, compiled `example_ores` server dylib, driven through a real `RcExecutorBuilder`/`RcExecutor`: a config-triggered panic in `example_ores:pulse_survey`'s tick hook is caught, the mod is disabled, the tick pipeline (multiple regions, repeated ticks) continues without the host test process itself ever panicking — the milestone's own acceptance criterion 2.
- [ ] `mod_reference_hook_dispatch.rs` proves every one of the reference mod's hooks (registry-build content, the generic tick hook, the client render-hook registration) fires at the point the corpus already fixes for it, with the correct data, headlessly — the milestone's own acceptance criterion 3; the client render hook's *visual* behavior remains explicitly, honestly deferred to M10 (PLAN-D2).
- [ ] `mod_reference_template_conformance.rs` proves `mods/example-ores/`'s own file layout matches MOD-D27's shared/server/client shape exactly.
- [ ] `reference_mod_persistence_roundtrip.rs` (in `rc-chunk-storage`) proves a chunk section containing `example_ores:pulse_crystal` block states survives a save/reload cycle whose *numeric* ids differ between the two boots, resolved correctly by namespaced name both times, and demonstrates the concrete `PaletteThresholds::blocks(n)` bit-width consequence of installing this one mod.
- [ ] `cargo run -p xtask -- lint-deps`/`fmt-check`/`lint` all exit 0 against the main workspace; the two `mods/` packages, being outside the main workspace, are invisible to and unconstrained by these checks (restated in Context).
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test` for the main workspace, plus explicit `cargo build`/`cargo test` invocations against both `mods/` manifests) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37/D43), on a clean checkout (TEST-D50).

## Context (self-contained)

### Why two new, small production edits are unavoidable, and why they are the *only* two

Reading M8-B01/B02/B03's own Context sections end to end (binding on this blueprint per `00-blueprint-spec.md`'s "restate everything the implementer needs") surfaces exactly two real gaps that block proving the milestone's acceptance criteria with genuine content, neither claimed by any other blueprint:

1. **`ModUpdateContext` (M8-B01, `block_behavior.rs`) has no public constructor.** Its five closure-bundle fields are declared with no `pub` modifier at all — crate-private to `rc-mod-api`. M8-B01's own Implementation step 10 explains why: at the time it was written, only `rc-mod-host` was ever expected to construct one, and `rc-mod-host` (M8-B02) explicitly never does — M8-B02's own Context states plainly it "never translates a mod's registration request into a real `bevy_ecs`... type" and never touches `BlockBehaviorRegistry` at all (that's `rc-mechanics`'s territory, out of reach for a crate that cannot depend on it, WS-D3). No blueprint through M8-B03 constructs a `ModUpdateContext` anywhere, in production code or in a test. This blueprint needs one — to unit-test `PulseCrystalBehavior` headlessly, exactly the kind of proof MOD-D29's future `rc-mod-test` crate will eventually generalize (not built here, a separate, later blueprint) — so this blueprint adds `impl<'a> ModUpdateContext<'a> { pub fn new(...) -> Self }`, mirroring the exact, permitted class of continuation M8-B02 already used for `RegistryBuildContext`/`ClientRegistryBuildContext` (new methods on an already-public type, zero changes to any already-declared signature).

2. **No `ModHookInvoke` implementation exists anywhere.** M8-B03's own `ModSystemShim` dispatch (Context, "Crash isolation and the disable-path") calls `(self.invoke)(ctx)` and reacts to `Err` — but `invoke: Arc<ModHookInvoke>` is, by M8-B03's own explicit design, "owned and constructed entirely by a future `rc-mod-host` blueprint," never `ModSystemShim` itself (Constraints (d) forbids `ModSystemShim` from ever calling `catch_unwind`). M8-B02, in turn, never touches `rc-scheduler`'s types at all — it has no dependency reach to `ModHookInvoke`/`ModTickInvocationCtx` and never claims to build this bridge either. Tracing the dependency graph settles where the real implementation *can* live: only `rc-scheduler` (which already depends on both `rc-mod-host` and `rc-mod-api`, per M8-B03's own `Cargo.toml` Deliverable) has legal reach to both `ModHookInvoke`'s own crate and `ServerModHost::call_on_tick_hook`'s crate — `rc-mod-host` itself cannot gain a dependency on `rc-scheduler` without creating a cycle (`rc-scheduler` already depends on `rc-mod-host`). This blueprint therefore adds `rc_scheduler::mod_host_bridge::native_mod_hook_invoke`, a small, new, additive, general-purpose (not `example_ores`-specific) function — the first concrete answer to a question three prior blueprints each deferred to "a future blueprint," here because this is the first blueprint that actually needs the answer to prove anything end to end.

Both edits are purely additive, change no already-shipped signature's shape, and are exercised entirely by this blueprint's own new tests — neither touches a pre-existing test file from any prior blueprint.

### What the reference mod does *not* yet get to do — restated honestly, inherited, not invented here

Two further limitations are inherited from M8-B01/B02/B03's own already-stated scope boundaries, restated here because this blueprint's own design must work *within* them, not around them:

- **`TickHookContext` (M8-B01) is a fieldless marker** — M8-B02's own completion gives it exactly `pub fn new() -> Self`, nothing else. A native mod's `on_tick_hook` body therefore has **no way to read or write a real ECS component through it** at M8 alpha, regardless of what its manifest declares. This blueprint's own generic hook (`example_ores:pulse_survey`, below) genuinely, correctly participates in the real conflict graph (`resolve_component_access`/`register_mod_system`/`compute_waves` all operate purely on the *declared* access set, never on whether a hook body actually used it) — but its own *observable* behavior is proven the same way M8-B02's own `good_mod` fixture already proves its tick hook ran: an env-var-selected log file, not a real component mutation. A future blueprint that gives `TickHookContext` real query methods extends `native_mod_hook_invoke`'s own body to thread `ctx.world`/`ctx.access` through at that point — no signature change to this blueprint's own bridge function, which already takes the full `ModTickInvocationCtx`.
- **Mod-registered-component ECS resolution does not exist.** M8-B03's own Context ("The engine-component export table," "named, binding limitation") states this plainly: `resolve_component_access` resolves only *engine*-exported names (`RcExecutorBuilder::export_component::<T>`), never a mod's own dynamically `register_component`-ed one — translating a recorded `ModComponentDescriptor` into a real, live `bevy_ecs::ComponentId` needs `World::register_component_with_descriptor` (ARCH-D4) plus a per-region replay mechanism mirroring `export_component`'s own, genuinely new engineering no prior blueprint built. This blueprint's `example_ores:ore_charge` component (registered via `RegistryBuildContext::register_component`, matching 06's own canonical worked example verbatim) is therefore proven **recorded** (via `ServerModHost::call_on_registry_build`, exactly M8-B02's own already-shipped, already-tested dispatch) but is **not** reachable by any `Access<ComponentId>`-based hook — this blueprint's own generic hook (`pulse_survey`) declares access only against two purpose-built, *engine*-exported test components instead (below), a deliberate, honest divergence from 06's illustrative manifest, not a silent gap.

### `mods/` lives outside the main workspace — mechanically, not by convention alone

`12-workspace-structure.md`'s `[workspace] members = ["crates/*", "xtask"]` is a **glob** — anything outside `crates/` is automatically excluded from the closed 28-crate manifest (WS-D2) with zero root-`Cargo.toml` edit needed. `mods/example-ores/Cargo.toml` is a **virtual manifest** (`[workspace] members = ["shared", "server", "client"]`, no `[package]` table of its own) — Cargo's own workspace-root discovery walks *up* from each member crate looking for the nearest ancestor `Cargo.toml` carrying a `[workspace]` table; since `mods/example-ores/Cargo.toml` is nearer than the repo root's, `shared`/`server`/`client` become members of *this* workspace, never the main one, with no `package.workspace` key needed on any of the three. `mods/conflict-probe/Cargo.toml` is a single-crate package carrying **both** `[package]` and an empty `[workspace]` table — the exact idiom M8-B02's own fixture crates already established ("a standalone Cargo package... its own `Cargo.toml` carrying an empty `[workspace]` table specifically to opt it out of the parent Rusty Clanker workspace"). Neither `mods/` package is ever built, linted, or lock-filed by the main workspace's own `cargo` invocations — every command this blueprint's own tests issue against them is an explicit, separate child-process `cargo build`/`cargo test --manifest-path mods/.../Cargo.toml` call, mirroring M8-B02's own `build_fixture_archive` technique exactly (Context, below). `.gitignore` gains one additive line, `/mods/**/target/`, since each of these three independent workspaces produces its own `target/` directory the main root's existing `/target/` entry does not cover.

### `example_ores`'s manifest, restated field-for-field against M8-B01's schema

Two committed native triples cover exactly this project's own CI matrix (TEST-D34/D37: `ubuntu-24.04`, `windows-2025`) — a real-world mod author targeting more platforms follows the identical pattern for more entries.

```toml
[mod]
id = "example_ores"
version = "0.1.0"
display_name = "Example Ores"
authors = ["Rusty Clanker Reference Mods"]
license = "MIT OR Apache-2.0"

[api]
requires = "^0.1"
unstable_features = []

[compat]
engine = ">=0.1.0, <0.2.0"
mc_parity = "26.2"

[dependencies]

[entrypoints]
tier = "native"
shared = "shared"
server = "example_ores_server_entry"
client = "example_ores_client_entry"

[entrypoints.native."x86_64-pc-windows-msvc"]
server = true
client = true

[entrypoints.native."x86_64-unknown-linux-gnu"]
server = true
client = true

[capabilities]
filesystem = false
network = false
network_channels = ["example_ores:sync"]

[[capabilities.components]]
hook = "example_ores:pulse_survey"
name = "rc_engine_test:pulse_flag"
access = "read"
group = "lighting"

[[capabilities.components]]
hook = "example_ores:pulse_survey"
name = "rc_engine_test:pulse_count"
access = "write"
group = "lighting"

[[hooks]]
id = "example_ores:pulse_survey"
group = "lighting"
before = []
after = []
exclusive_world_access = false
```

`[[hooks]]` carries exactly one entry — the pulse crystal's own scheduled-tick behavior is registered through `RegistryBuildContext::register_block_behavior` inside `on_registry_build`, **not** through a `[[hooks]]` declaration at all (M8-B01's own Context: "a mod that gives one new block custom `on_scheduled_tick`... behavior does not need its own `ModSystemShim`-style Stage-4 hook... no new conflict-graph entry, no new `[[hooks]]` manifest declaration"). `pulse_survey`'s own `group = "lighting"` (not `block_redstone`) is a deliberate choice: it keeps the two mechanisms (block behavior vs. generic hook) visibly, structurally separate in this one mod, and is thematically apt — real vanilla lighting already reads block state to compute propagation, so a Lighting-group hook reading an engine-exported block-state-shaped marker and writing a per-entity pulse counter is a sensible (if illustrative) shape, not an arbitrary one.

### `example_ores:pulse_crystal` — the block, its ids, and the WORLD-D2 palette-bit-width consequence, with real numbers

Two states, one boolean property (`lit`, values `"false"`/`"true"`, vanilla's own string-boolean convention for block-state properties). Ids are allocated **densely, immediately after the pinned target's own highest vanilla id** (M8-B01's own binding resolution of MOD-D6, "Id allocation must be dense and sequential"), via `rc_mod_api::registry::DenseIdAllocator`, exactly as M8-B01's own `registry_ids.rs` already proves that type's behavior in isolation. This blueprint's own registry-build test (below) allocates from `DenseIdAllocator::starting_at(0)` (mirroring M8-B02's own already-established `RegistryBuildContext::new(0, 0)` test convention, since only ordering-within-this-mod matters at that scope, not absolute values) — a **separate**, illustrative test (`rc-chunk-storage`'s own persistence-roundtrip suite) instead picks realistic, production-shaped numbers to make WORLD-D2's own consequence concrete:

- **Boot A:** a synthetic pinned-target registry size of `1023` (ids `0..1022`) is "already installed" when `example_ores` boots; its `DenseIdAllocator::starting_at(1023)` assigns `pulse_crystal[lit=false] = 1023`, `pulse_crystal[lit=true] = 1024`. `PaletteThresholds::blocks(direct_bits)`'s own `direct_bits = ceil_log2(registry_size)`: before this mod, `ceil_log2(1023) = 10`; after, `ceil_log2(1025) = 11` — every `Direct`-palette section's own bit width grows by exactly one bit the instant this one small mod is installed, independent of any one section's own local distinct-value count, exactly the consequence M2-B01's own Context narrates in prose ("inflating every `Direct`-palette section's bit width for the rest of the process's life, even for a world with a single mod block installed") — made numeric here.
- **Boot B:** the *same* two block states, resolved by their namespaced name (`"example_ores:pulse_crystal"` + `{"lit": "false"|"true"}`) through a **different** synthetic `BlockStateNames` resolver whose numeric assignment reflects a hypothetically-different load order (`pulse_crystal[lit=false] = 1030`, `[lit=true] = 1031`, e.g. as if one more small mod had loaded before it) — proving MOD-D6's own dense-allocation limitation's flip side is exactly what M2-B04's namespaced-name persistence already handles for free: **the persisted identity is the name, never the number** (Context, next subsection).

### Persistence: block-state identity is the namespaced name, never the boot-assigned number

M2-B04's `ChunkNbtCodec`'s `BlockStateNames` trait (Context there: "Registry-id resolver seam") is the entire mechanism: a chunk's on-disk palette entries store a block's namespaced id plus sorted property key/value strings, **never** a raw `BlockStateId(u32)` — the numeric id exists only in memory, resolved fresh, in both directions, by whatever `BlockStateNames` implementation the caller supplies at load time. This blueprint's own persistence test constructs two **different**, hand-authored mock `BlockStateNames` implementations (Boot A's and Boot B's own id assignments above) and proves: `to_nbt` under Boot A's resolver, `from_nbt` under Boot B's resolver, on the *same* NBT bytes, yields a `BlockStateColumn` whose *positions* hold Boot B's own numeric ids (`1030`/`1031`) even though every byte on disk was written under Boot A's numbers (`1023`/`1024`) — this is 06's own "stable-id mapping across restarts" promise (MOD-D6's own dense-allocation text: adding/removing/reordering a mod changes subsequently-loaded mods' ids) made concrete and mechanically proven, not merely asserted in prose. A future blueprint that gives `rc-mod-host` a real, resolvable per-boot `BlockStateNames` implementation (bridging `DenseIdAllocator`'s own run-time assignments into this trait) is the missing piece that makes this mechanism reachable in a real running server — not built here (out of this blueprint's own crate-dependency reach: `rc-chunk-storage` does not, and per WS-D3 rule 2 should not, depend on `rc-mod-api`) — this blueprint's own test supplies both resolvers by hand, exactly mirroring M2-B04's own already-established "a small, synthetic, hand-authored mock registry" test convention.

### The vanilla-client compatibility stance — stated honestly, since neither `06` nor `07` names one

No planning document fixes what a real, unmodified vanilla Java Edition client sees when it receives a block-state id outside its own compiled-in registry range. This blueprint's own, binding statement, since none exists elsewhere to restate: **running a real vanilla Java client against a modded Rusty Clanker world is not a supported configuration, and no compatibility mechanism for it exists anywhere in this project's design.** `NET-D1` pins protocol 776 against one specific, fixed vanilla registry shape; a mod's block/item ids, allocated densely just past that registry's own highest id (Context, above), are a Rusty-Clanker-only extension no real Mojang server or client has ever produced or consumed — a real vanilla client's own local, compiled-in block palette simply has no entry for such an id, and its behavior on receiving one is unspecified by this project (and not this project's problem to define, since it is not a client this project ships or supports). The **only** client this project ever expects to correctly render `example_ores:pulse_crystal` is Rusty Clanker's own Phase 2 native client — which does not exist yet (`M9`/`M10`, `PLAN-D2`) — and even once it does, only once it loads this same mod's `client` entrypoint itself (MOD-D5's isomorphic loading, exercised by this blueprint's own client-side tests, below). At M8 alpha, **no client anywhere can render this block** — the milestone's own acceptance criterion 3 already names this precisely ("client-side render hook visual verification explicitly deferred to `M10`... at M8 it is registered + headless-verified only"), and this blueprint's own client-side proof stops at exactly that line, never further.

### The item — registration only, deliberately unobtainable at M8 alpha

`example_ores:pulse_shard` (`max_stack_size = 16`) is registered via `RegistryBuildContext::register_item` inside `on_registry_build`, proving MOD-D6's registry-insertion seam for the `Item` kind. It is **not obtainable by any in-game means at M8 alpha** — no crafting-recipe system, no loot-table system, no creative-inventory tab exists anywhere in this milestone's own dependency reach (`05-game-mechanics.md`'s content is `M3`/`M4` scope for *native* content; the mod-facing equivalents are all still contract-only per `06`'s own Interfaces section). It carries no NBT component data (`ItemStackRecord.components: None`, matching M4-B01's own field shape restated in this blueprint's Prerequisites) — no item-component system exists yet for it to carry anything meaningful. This blueprint's own test proves only that the registration call is recorded correctly (`ServerModHost::call_on_registry_build`'s already-shipped dispatch, M8-B02) — obtainability is a future blueprint's problem, once a real crafting/loot/creative-tab mechanism exists for a mod to plug into.

### Client extension point — registered, headless-verified, per PLAN-D2

`example_ores`'s client entry calls `ClientRegistryBuildContext::register_block_renderer(Identifier::parse("example_ores:pulse_crystal").unwrap())` inside `on_client_registry_build` — proving MOD-D18's client-side registry seam exists and is reached through this mod's own isomorphic client entrypoint. This blueprint's own test (`ClientModHost::call_on_client_registry_build`, M8-B02's own already-shipped dispatch) asserts `ClientRegistryBuildContext::registrations()` contains exactly this one `ClientRegistration::BlockRenderer` entry — the milestone's own "registered + headless-verified only" language, realized literally: no renderer exists to draw anything, and this blueprint builds none.

### Proving isomorphism — the same compiled logic, invoked from both sides, checkably

`shared/src/lib.rs` owns the pulse crystal's own redstone-observable state-transition logic:

```rust
/// How many ticks between one pulse and the next (`example_ores:pulse_survey` does not
/// use this constant — it belongs to the block-behavior path, `PulseCrystalBehavior`).
pub const PULSE_PERIOD_TICKS: u64 = 40;

/// Pure state-transition function: given the crystal's current `lit` value, returns the
/// next `lit` value plus the `event_param` byte `emit_block_event` should carry (vanilla's
/// own block-event convention: an arbitrary, behavior-defined byte, here simply the next
/// `lit` value widened to `u8`). Deliberately trivial (a toggle) — the point of this
/// function is that it is the *one* place this logic is ever written, not that the logic
/// itself is interesting.
pub fn next_pulse_event(current_lit: bool) -> (bool, u8) {
    let next_lit = !current_lit;
    (next_lit, next_lit as u8)
}
```

`server`'s `PulseCrystalBehavior::on_scheduled_tick` calls it to decide the crystal's own next state (Context, below). `client`'s `on_client_init` **also** calls it once, unconditionally, against a fixed known input (`false`), and writes the result (a fixed, checkable value: `(true, 1)`) to a log file named by an env var, exactly mirroring M8-B02's own already-established `RC_MOD_HOST_FIXTURE_LOG_PATH` file-log signaling convention (its own Context: "the dylib and the test process share one OS process's environment once loaded... this blueprint's own simple, portable, no-extra-FFI signaling mechanism"), reused here under this blueprint's own env var name, `EXAMPLE_ORES_FIXTURE_LOG_PATH`. A test asserts **both** the server-side unit test (Context, below) and the client-side dylib-dispatch test independently observe the identical `next_pulse_event(false) == (true, 1)` result — proving the *same compiled logic* actually executed on both sides, not merely that both crates happen to depend on the same source.

### The panic trigger — config, not a separate fixture crate

`example_ores:pulse_survey`'s `on_tick_hook` dispatch (inside `server`'s own `ServerModEntry` implementation) checks `std::env::var_os("EXAMPLE_ORES_FORCE_PANIC").is_some()` at the top of its own body and, if set, `panic!("example_ores: deliberate config-triggered panic")` unconditionally before doing anything else. Every other call path (registry-build, other tick invocations, channel/mod messages) ignores this variable entirely. This is the milestone's own literal wording realized precisely ("the reference mod's tick hook is made to panic deliberately... a config-triggered panic in the tick hock") — the *same* compiled `example_ores` dylib is used, unmodified, for every one of this blueprint's own tests; only the test process's own environment differs between "everything works" and "crash isolation" runs, mirroring the portable, no-extra-FFI signaling discipline M8-B02 already established for its own fixtures.

### `conflict_probe` — the second mod, and exactly what it conflicts on

A minimal, single-crate, server-only native mod. Its own manifest declares one hook in the *same* domain group as `example_ores:pulse_survey` (`lighting`), with an explicit `after` reference to it:

```toml
[[hooks]]
id = "conflict_probe:counter_tick"
group = "lighting"
before = []
after = ["example_ores:pulse_survey"]
exclusive_world_access = false
```

Loaded **alone**, `conflict_probe` is a perfectly valid, loadable mod — its own `resolve_hook_order` call (with no sibling hooks present) places it cleanly after the `NATIVE` anchor, no error (M8-B03's own default-after-native rule, Context there). The **rejection** this blueprint proves (milestone acceptance criterion 1) needs a *second*, test-only manifest variant of `example_ores` itself — never committed as a second file, built at test time by string-editing the one canonical, shipped `manifest.toml`'s text (mirroring M8-B01's own established `MINIMAL_MANIFEST`-plus-string-editing test technique exactly): the same `pulse_survey` `[[hooks]]` block, with one line appended, `after = ["conflict_probe:counter_tick"]`. Packed into a *separate* `.rcmod` archive around the **same**, already-built `example_ores` server binary (Context, "Fixture build mechanism," below — no second compile needed), this manifest variant plus `conflict_probe`'s own real manifest together form the exact two-hook cycle M8-B03's own `resolve_hook_order` already proves it rejects (`mod_conflict_graph_integration.rs` test 4's own synthetic pair, now realized with two real, separately-compiled, `ServerModHost`-loaded dylibs): `pulse_survey` after `counter_tick`, `counter_tick` after `pulse_survey`. The canonical, normally-shipped `manifest.toml` (no such line) is what every *other* test in this blueprint's suite uses — `example_ores` loaded alone is never rejected.

### Fixture build mechanism — the same technique M8-B02 already established, applied to two real mods

`crates/scheduler/tests/common/mod_fixture.rs` (new) provides:

```rust
/// Runs `cargo build --manifest-path {crate_dir}/Cargo.toml` as a child process
/// (target dir: `{CARGO_TARGET_TMPDIR}/mod-builds/{crate_dir's own dir name}` — no
/// `tempfile` crate needed, mirroring M8-B02's own established convention), locates
/// the produced `cdylib` via `std::env::consts::{DLL_PREFIX, DLL_SUFFIX}`, and packs
/// it plus `manifest_toml` into a fresh `.rcmod` zip archive under
/// `native/{rc_mod_host::CURRENT_TARGET_TRIPLE}/{mod_id}.{ext}` (`rc_mod_host::
/// native_binary_filename`'s own MOD-D4-shaped convention, reused directly — this
/// crate already depends on `rc-mod-host`). Cached per-process by `crate_dir`, so
/// packing the *same* already-built binary under two different `manifest_toml`
/// strings (Context: "conflict_probe") costs one compile, not two.
pub fn build_and_package_mod(crate_dir: &std::path::Path, mod_id: &str, manifest_toml: &str) -> std::path::PathBuf;

/// `rc_mod_host::sha256_hex` of the archive's own packed native-binary bytes,
/// exposed so a test can build the matching `NativeTrustEntry` (MOD-D26) — the same
/// call M8-B02's own `trust_allowlist.rs`/`handshake_matrix.rs` already make.
pub fn native_binary_sha256(archive_path: &std::path::Path, mod_id: &str) -> String;
```

`crates/scheduler/Cargo.toml` gains one new `[dev-dependencies]` entry, `zip = { workspace = true }` (already pinned, `12`'s `[workspace.dependencies]`; already `rc-mod-host`'s own second consumer per M8-B02's own precedent — this blueprint is a third, dev-only consumer, writing archives where `rc-mod-host` only ever reads them).

### The end-to-end harness — real `ServerModHost` + real `RcExecutorBuilder`, driven by `native_mod_hook_invoke`

```rust
// crates/scheduler/src/mod_host_bridge.rs (new production module)
use std::sync::Arc;
use rc_mod_api::{Identifier, ModId};
use rc_mod_host::ServerModHost;
use crate::mod_system::{ModHookFailure, ModHookInvoke, ModTickInvocationCtx};

/// The first real `ModHookInvoke` implementation (Context: "no `ModHookInvoke`
/// implementation exists anywhere"). Dispatches `hook_id` on `mod_id` through
/// `host`'s own already-crash-isolated `call_on_tick_hook` (M8-B02) and translates
/// its `HookOutcome` into this crate's `Result` shape. Never calls `catch_unwind`
/// itself (M8-B03 Constraints (d)) — `host.call_on_tick_hook` already performed that
/// catch before this closure's own `match` ever runs. `ctx.world`/`ctx.access` are
/// deliberately unused (Context: "`TickHookContext` is a fieldless marker") — this
/// is the honest, current-scope reason, not an oversight; a future blueprint that
/// gives `TickHookContext` real accessors extends this function's body, not its
/// signature. General-purpose: works for any loaded mod/hook pair, not hardcoded to
/// `example_ores`.
pub fn native_mod_hook_invoke(
    host: Arc<ServerModHost>,
    mod_id: ModId,
    hook_id: Identifier,
) -> Arc<ModHookInvoke> {
    let hook_id_str = hook_id.to_string();
    Arc::new(move |_ctx: ModTickInvocationCtx<'_>| -> Result<(), ModHookFailure> {
        use rc_mod_host::HookOutcome;
        let mut tick_ctx = rc_mod_api::TickHookContext::new();
        match host.call_on_tick_hook(&mod_id, &hook_id_str, &mut tick_ctx) {
            HookOutcome::Ran(stabby_result) => match stabby_result {
                stabby::result::Result::Ok(()) => Ok(()),
                stabby::result::Result::Err(e) => Err(ModHookFailure { reason: e.to_string() }),
            },
            HookOutcome::Panicked { message } => Err(ModHookFailure { reason: message }),
            HookOutcome::Skipped => Err(ModHookFailure { reason: "mod already disabled".to_string() }),
        }
    })
}
```

`crates/scheduler/src/lib.rs` gains `mod mod_host_bridge; pub use mod_host_bridge::native_mod_hook_invoke;` — additive, alongside M8-B03's own three existing `mod`/`pub use` lines, none of which change.

The harness itself (`crates/scheduler/tests/common/mod_fixture.rs`, continued): builds `example_ores`'s server dylib and `conflict_probe`'s dylib via `build_and_package_mod`; constructs a `ModHostConfig { mods_dir, native_trust: vec![...], fault_policy: ModFaultPolicy::Disable }` naming both archives' own `native_binary_sha256`; calls `ServerModHost::discover_and_load` (real discovery, real handshake, real registry-build dispatch — M8-B02, unmodified); wraps the returned host in `Arc`; constructs `RcExecutorBuilder::new(bootstrap)` where `bootstrap` inserts two plain `bevy_ecs::Component` marker types (`PulseFlag(pub bool)`, `PulseCount(pub u32)`) via `.export_component::<PulseFlag>(id("rc_engine_test:pulse_flag"))` / `.export_component::<PulseCount>(id("rc_engine_test:pulse_count"))`; resolves `pulse_survey`'s declared `ComponentAccessDecl`s via `resolve_component_access` against those two exports; calls `.register_mod_system(mod_id("example_ores"), id("example_ores:pulse_survey"), rc_mod_api::DomainGroup::Lighting, declared_access, false, Vec::new(), Arc::new(AtomicBool::new(false)), native_mod_hook_invoke(Arc::clone(&host), mod_id("example_ores"), id("example_ores:pulse_survey")))`; `.build()`; `spawn_region`s one or more regions; drives `tick_region` via a plain loop (never `std::thread::sleep`-gated — a tight loop asserting each call's own wall-clock duration stays well under the 50 ms/tick budget is sufficient and flake-free, restated below) against a minimal, hand-rolled `NoopTransport: rc_messaging::Transport` test double (never the real `InProcessTransport` — that crate lives in `NETRENDER`, unreachable from `rc-scheduler`'s own `SIM` membership, WS-D3 Rule 2; mirroring M0-B05's own already-established `MockTransport`-in-test-file convention exactly) and one `RcWorkerPool::new(2)`.

### "Continues at 20 TPS" — restated against `tick_region`'s own already-documented shape

M0-B05's own `tick_region` doc comment states plainly it is "the synchronous test-mode tick driver... bypassing real-time EDF admission entirely; a later blueprint wraps this in the wall-clock-paced, multi-region 20 TPS loop (out of scope here)." That wall-clock-paced driver does not exist yet, anywhere in the committed corpus — this blueprint does not build it (out of scope, restated in Constraints). This blueprint's own crash-isolation test instead proves the property the milestone's own criterion actually cares about, honestly scoped to what `tick_region` already gives: repeated `tick_region` calls (this blueprint's own test drives at least 25, one nominal second's worth at the 50 ms/tick budget `01`'s own pipeline already fixes) against **two** regions — one carrying `example_ores`'s `pulse_survey` shim (which panics once `EXAMPLE_ORES_FORCE_PANIC` is set, is disabled, and is a no-op every tick after), one carrying only an ordinary, always-succeeding native system — complete successfully, each individually well under 50 ms, with the region's own `tick_counter` advancing by exactly one per call, for every tick both before and after the disabling panic. The real, wall-clock-paced, multi-region driver's own eventual construction remains, honestly, a later blueprint's job.

## Deliverables

### `mods/example-ores/Cargo.toml` (new — virtual workspace root)

```toml
[workspace]
members = ["shared", "server", "client"]
resolver = "2"
```

### `mods/example-ores/manifest.toml` (new)

Exactly the TOML shown in Context, "`example_ores`'s manifest."

### `mods/example-ores/shared/Cargo.toml` (new)

```toml
[package]
name = "example-ores-shared"
version = "0.1.0"
edition = "2021"

[lib]
name = "example_ores_shared"
```

### `mods/example-ores/shared/src/lib.rs` (new)

Exactly the two items shown in Context, "Proving isomorphism": `PULSE_PERIOD_TICKS`, `next_pulse_event`.

### `mods/example-ores/shared/tests/next_pulse_event.rs` (new)

1. `toggles_from_false` — `next_pulse_event(false) == (true, 1)`.
2. `toggles_from_true` — `next_pulse_event(true) == (false, 0)`.
3. `is_its_own_inverse_applied_twice` — for both booleans `b`, `next_pulse_event(next_pulse_event(b).0).0 == b`.

### `mods/example-ores/server/Cargo.toml` (new)

```toml
[package]
name = "example-ores-server"
version = "0.1.0"
edition = "2021"

[lib]
name = "example_ores_server"
crate-type = ["cdylib", "rlib"]

[dependencies]
example-ores-shared = { path = "../shared" }
rc-mod-api = { path = "../../../crates/mod-api", default-features = false, features = ["native-tier"] }
stabby = "72.1.16"
```

(`crate-type` carries `rlib` alongside `cdylib` — a standard, additional Cargo output alongside the real `.dll`/`.so`/`.dylib` MOD-D4's mod loader ever opens, purely so `cargo test -p example-ores-server` can link this crate directly for its own unit tests, below; the runtime artifact MOD-D4 names is unaffected. `stabby`'s own version is pinned here explicitly, matching MOD-D3 exactly — this package is outside `12`'s `[workspace.dependencies]` inheritance, being outside the main workspace, Context.)

### `mods/example-ores/server/src/lib.rs` (new)

```rust
//! `example_ores`'s server entry: registers `pulse_crystal` (block + behavior),
//! `pulse_shard` (item), `ore_charge` (component, recorded-only — Context), the
//! `example_ores:sync` channel, and the `pulse_survey` generic tick hook. Every
//! item here is real, working content — not a stub (the milestone's own framing).

use example_ores_shared::{next_pulse_event, PULSE_PERIOD_TICKS};
use rc_mod_api::{
    BlockRegistration, ComponentDescriptorBuilder, Identifier, ItemRegistration, ModAbiVersion,
    ModBlockBehavior, ModBlockPos, ModBlockStateId, ModHookError, ModInitError, ModUpdateContext,
    RegistryBuildContext, ServerModEntry, TickHookContext, TickPriority, MOD_API_VERSION,
};

/// Vanilla-shaped: two states, one `Direction`/`ModBlockPos`-scoped tick behavior,
/// zero fields beyond the two ids it needs to interpret its own current state
/// (Context: MOD-D17 permits fixed, learned-once-at-boot config; it forbids
/// mutable per-call state, of which this struct holds none).
struct PulseCrystalBehavior {
    off: ModBlockStateId,
    on: ModBlockStateId,
}

impl ModBlockBehavior for PulseCrystalBehavior {
    fn on_scheduled_tick(&self, ctx: &mut ModUpdateContext, pos: ModBlockPos) {
        // NOTE: `ModUpdateContext`'s own public accessor methods (`get_block`, `set_block`,
        // `schedule_block_tick`, `emit_block_event`) use plain `std::option::Option`/`bool`
        // and take `&mut self`, per M8-B01's own Deliverables — distinct from the
        // `stabby`-wrapped types that cross the trait-vtable boundary (`ModBlockBehavior`'s
        // own `on_shape_update` return, for instance). `ctx` above is therefore `&mut
        // ModUpdateContext`, matching that `&mut self` requirement.
        let current_lit = ctx.get_block(pos) == Some(self.on);
        let (next_lit, event_param) = next_pulse_event(current_lit);
        let next_id = if next_lit { self.on } else { self.off };
        ctx.set_block(pos, next_id);
        ctx.emit_block_event(pos, /* PULSE_EVENT_ID */ 1, event_param, next_id);
        ctx.schedule_block_tick(pos, PULSE_PERIOD_TICKS, TickPriority::Normal);
    }
}

#[derive(Default)]
struct ExampleOresServerEntry;

impl ServerModEntry for ExampleOresServerEntry {
    fn on_registry_build(&mut self, ctx: &mut RegistryBuildContext) -> stabby::result::Result<(), ModInitError> {
        let off = ctx.register_block(BlockRegistration {
            id: "example_ores:pulse_crystal".into(), default_state_component_count: 2,
        });
        let on = ctx.register_block(BlockRegistration {
            id: "example_ores:pulse_crystal".into(), default_state_component_count: 2,
        });
        ctx.register_block_behavior(off, /* dynptr!(Box::new(PulseCrystalBehavior{off,on})) */);
        ctx.register_block_behavior(on, /* dynptr!(Box::new(PulseCrystalBehavior{off,on})) */);
        let _item = ctx.register_item(ItemRegistration { id: "example_ores:pulse_shard".into(), max_stack_size: 16 });
        let descriptor = ComponentDescriptorBuilder::new("example_ores:ore_charge", 4, 4).unwrap().build().unwrap();
        let _component = ctx.register_component(descriptor);
        ctx.register_channel(Identifier::parse("example_ores:sync").unwrap());
        stabby::result::Result::Ok(())
    }

    fn on_tick_hook(&mut self, hook_id: &stabby::string::String, _ctx: &mut TickHookContext) -> stabby::result::Result<(), ModHookError> {
        if hook_id.as_str() == "example_ores:pulse_survey" {
            if std::env::var_os("EXAMPLE_ORES_FORCE_PANIC").is_some() {
                panic!("example_ores: deliberate config-triggered panic");
            }
            if let Ok(path) = std::env::var("EXAMPLE_ORES_FIXTURE_LOG_PATH") {
                use std::io::Write;
                if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
                    let _ = writeln!(f, "pulse_survey");
                }
            }
        }
        stabby::result::Result::Ok(())
    }
}

#[stabby::export]
extern "C" fn rc_mod_abi_handshake() -> ModAbiVersion { MOD_API_VERSION }

#[stabby::export]
extern "C" fn example_ores_server_entry() -> stabby::dynptr!(stabby::boxed::Box<dyn ServerModEntry>) {
    /* dynptr!(Box::new(ExampleOresServerEntry::default()) as Box<dyn ServerModEntry>) */
    unimplemented!("exact stabby::dynptr! construction syntax — moderate-confidence flag, Constraints")
}
```

(`register_block_behavior`'s `dynptr!` construction and the entry-factory export's own `dynptr!` construction are both left as commented pseudocode / an explicit `unimplemented!` marker — M8-B02's own Context already flags `dynptr!`'s exact mod-author-side construction syntax as an "elevated moderate-confidence" item to confirm against installed `stabby` 72.1.16 docs before any fixture crate is written; this blueprint inherits that exact flag rather than re-deriving it, restated in Constraints. No Deliverable *signature* changes if the resolved syntax differs — only these two call sites' own bodies.)

### `mods/example-ores/server/tests/pulse_crystal_behavior.rs` (new — the mod's own headless behavior-unit suite)

Constructs a `ModUpdateContext` via this blueprint's own new `ModUpdateContext::new` constructor (Context, Deliverables below), backed by simple `Cell`/`RefCell`-captured recording closures — no dylib, no `rc-mod-host`, no `ServerModHost` involved at all.

1. `off_toggles_to_on_and_reschedules` — a `PulseCrystalBehavior { off: ModBlockStateId(10), on: ModBlockStateId(11) }`; a `ModUpdateContext` whose `get_block` closure always returns `Some(ModBlockStateId(10))` and whose `set_block`/`schedule_block_tick`/`emit_block_event` closures each record their own call arguments into a shared `RefCell<Vec<..>>`; call `on_scheduled_tick`; assert `set_block` was called exactly once with `(pos, ModBlockStateId(11))`, `emit_block_event` was called with `event_param == 1`, and `schedule_block_tick` was called with `delay_ticks == PULSE_PERIOD_TICKS`, `priority == TickPriority::Normal`.
2. `on_toggles_to_off` — the mirror of test 1, `get_block` returns `Some(ModBlockStateId(11))`; asserts `set_block(pos, ModBlockStateId(10))`, `event_param == 0`.
3. `absent_block_is_treated_as_off` — `get_block` returns `None`; asserts identical behavior to test 1 (a crystal with no prior recorded state starts from "off").
4. `shared_logic_matches_the_hand_computed_table` — for both `current_lit` inputs, `next_pulse_event(current_lit)` (called directly from `example_ores_shared`) matches exactly what test 1/2's own recorded `set_block`/`emit_block_event` arguments implied — a direct cross-check that the behavior's own dispatch and the shared crate's own pure function agree.

### `mods/example-ores/server/tests/registry_build_recording.rs` (new)

Constructs a bare `ExampleOresServerEntry::default()` and a `RegistryBuildContext::new(0, 0)` directly (both fully public per M8-B01/B02); calls `on_registry_build`.

1. `registers_exactly_two_block_states_for_one_block_id` — `into_recorded().blocks` has length 2, both entries' `BlockRegistration.id == "example_ores:pulse_crystal"`, `ModBlockStateId`s are `0` and `1` (M8-B02's own already-proven `DenseIdAllocator` sequential-from-zero behavior).
2. `registers_exactly_one_item` — `into_recorded().items` has length 1, `id == "example_ores:pulse_shard"`, `max_stack_size == 16`.
3. `registers_exactly_one_component_named_ore_charge` — `into_recorded().components` has length 1; the recorded `ModComponentDescriptor.name.as_str() == "example_ores:ore_charge"`, `size == 4`, `align == 4`.
4. `registers_exactly_two_block_behaviors_keyed_to_their_own_state_ids` — `into_recorded().behaviors` has length 2, keyed `ModBlockStateId(0)` and `ModBlockStateId(1)`.
5. `registers_exactly_one_channel` — `into_recorded().channels == [Identifier::parse("example_ores:sync").unwrap()]`.
6. `returns_ok` — `on_registry_build`'s own return value is `stabby::result::Result::Ok(())`.

### `mods/example-ores/client/Cargo.toml` (new)

Identical shape to `server/Cargo.toml`, package name `example-ores-client`, lib name `example_ores_client`, no dependency on `example-ores-shared`'s server-side neighbor beyond the shared crate itself.

### `mods/example-ores/client/src/lib.rs` (new)

```rust
//! `example_ores`'s client entry: registers the block renderer extension point
//! (MOD-D18, headless-verified only — Context) and proves isomorphism by invoking
//! the identical shared logic the server side uses.

use example_ores_shared::next_pulse_event;
use rc_mod_api::{
    ClientInitContext, ClientModEntry, ClientRegistryBuildContext, Identifier, ModInitError,
    ModAbiVersion, MOD_API_VERSION,
};

#[derive(Default)]
struct ExampleOresClientEntry;

impl ClientModEntry for ExampleOresClientEntry {
    fn on_client_registry_build(&mut self, ctx: &mut ClientRegistryBuildContext) -> stabby::result::Result<(), ModInitError> {
        ctx.register_block_renderer(Identifier::parse("example_ores:pulse_crystal").unwrap());
        stabby::result::Result::Ok(())
    }

    fn on_client_init(&mut self, _ctx: &mut ClientInitContext) -> stabby::result::Result<(), ModInitError> {
        let (next_lit, event_param) = next_pulse_event(false);
        if let Ok(path) = std::env::var("EXAMPLE_ORES_FIXTURE_LOG_PATH") {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
                let _ = writeln!(f, "client_init lit={next_lit} param={event_param}");
            }
        }
        stabby::result::Result::Ok(())
    }
}

#[stabby::export]
extern "C" fn rc_mod_abi_handshake() -> ModAbiVersion { MOD_API_VERSION }

#[stabby::export]
extern "C" fn example_ores_client_entry() -> stabby::dynptr!(stabby::boxed::Box<dyn ClientModEntry>) {
    unimplemented!("dynptr! construction syntax — same flag as server/src/lib.rs")
}
```

### `mods/conflict-probe/Cargo.toml`, `manifest.toml`, `src/lib.rs` (new)

Single-crate package (`[package]` + empty `[workspace]`, mirroring M8-B02's own fixture idiom), `crate-type = ["cdylib"]` (no `rlib` — this mod has no unit-tested logic of its own, only an ordering-cycle probe). `src/lib.rs` mirrors `example_ores/server/src/lib.rs`'s own handshake/entry-factory shape exactly, with an `on_tick_hook` body that unconditionally does nothing (`Ok(())`) for `hook_id == "conflict_probe:counter_tick"` — it never needs to actually run in any of this blueprint's own tests, since the rejection this mod exists to prove happens at `resolve_hook_order`, strictly before `register_mod_system`/dispatch is ever reached (Context, M8-B03's own already-established "which stage rejects" precedent, restated).

### `crates/mod-api/src/block_behavior.rs` (modify — one additive constructor)

```rust
impl<'a> ModUpdateContext<'a> {
    /// New (this blueprint) — the missing public constructor (Context: "why two new,
    /// small production edits are unavoidable," item 1). Parameter types are exactly
    /// this struct's own already-declared private field types, unchanged; a caller
    /// (this blueprint's own test suite, or a future `rc-mod-host` blueprint) supplies
    /// real or test-double closures. `current_tick` is the struct's one plain `pub`
    /// field, set directly.
    pub fn new(
        get_block: stabby::closure::CallMut1<'a, ModBlockPos, stabby::option::Option<ModBlockStateId>>,
        set_block: stabby::closure::CallMut2<'a, ModBlockPos, ModBlockStateId, bool>,
        schedule_block_tick: stabby::closure::CallMut3<'a, ModBlockPos, u64, TickPriority, ()>,
        schedule_fluid_tick: stabby::closure::CallMut3<'a, ModBlockPos, u64, TickPriority, ()>,
        emit_block_event: stabby::closure::CallMut4<'a, ModBlockPos, u8, u8, ModBlockStateId, ()>,
        current_tick: u64,
    ) -> Self {
        Self { get_block, set_block, schedule_block_tick, schedule_fluid_tick, emit_block_event, current_tick }
    }
}
```

**Moderate-confidence flag, inherited and elevated.** M8-B01's own Context already flags `stabby::closure::Call*`/`CallMut*`'s exact generic-parameter spelling as unconfirmed; this blueprint's own research pass (`docs.rs/stabby/72.1.16/stabby/closure/`) found the family described as **traits** ("`stabby::closure` exports the `CallN`, `CallMutN` and `CallOnceN` traits... as ABI-stable equivalents of `Fn`, `FnMut` and `FnOnce`"), not confirmed as the concrete generic structs M8-B01's own field declarations assume a struct field could hold directly. If confirmed at implementation time that these are trait-only (no directly-instantiable concrete type), **both** M8-B01's own already-shipped field declarations in `block_behavior.rs` **and** this blueprint's own `new` constructor above need reconciling to whatever concrete, storable type (a boxed trait object, a `stabby::closure`-provided wrapper struct implementing the trait, or similar) `stabby` 72.1.16 actually exposes for this purpose — a correction to M8-B01's own Deliverables this blueprint's implementer must make hand-in-hand with this one, not something this blueprint can resolve by web research alone. This does not change any *other* Deliverable signature in this blueprint.

### `crates/scheduler/Cargo.toml` (modify — one additive dev-dependency)

Add `zip = { workspace = true }` under `[dev-dependencies]`; every existing line (Deliverables, M8-B03) unchanged.

### `crates/scheduler/src/mod_host_bridge.rs` (new)

Exactly as shown in Context, "The end-to-end harness."

### `crates/scheduler/src/lib.rs` (modify — one additive module + re-export line)

```rust
mod mod_host_bridge;
pub use mod_host_bridge::native_mod_hook_invoke;
```

### `crates/scheduler/tests/common/mod_fixture.rs` (new)

Exactly the two functions shown in Context, "Fixture build mechanism," plus a minimal `struct NoopTransport;` implementing `rc_messaging::Transport` (send is a no-op `Ok(())`; no inbound messages are ever produced) — this blueprint's own tests need no real cross-region delivery.

### `.gitignore` (modify — one additive line)

Add `/mods/**/target/` to the existing pattern list; every existing line (M0-B01) unchanged.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file below, the two `mods/` crate trees in full (real, complete, working source — these are content this blueprint delivers, not implementation to stub, mirroring M8-B02's own fixture-crate precedent: "real, complete, working source... these are test *inputs*, not implementation"), the `rc-mod-api`/`rc-scheduler` production-file edits from Deliverables with their new function bodies replaced by `todo!()`, and the `Cargo.toml`/`lib.rs`/`.gitignore` edits. The implementation changeset fills `todo!()` bodies only; it must not modify any file under `mods/`, `crates/scheduler/tests/`, `crates/mod-api/tests/`, or `crates/chunk-storage/tests/`, and must not weaken any assertion below.

### `crates/scheduler/tests/mod_reference_conflict_graph.rs` (new)

1. `example_ores_loads_alone_and_registers_into_a_real_conflict_graph` — build+load `example_ores` (canonical manifest) via `ServerModHost::discover_and_load`; `RcExecutorBuilder` with both engine-exported test components + `register_mod_system` for `pulse_survey` (declared access resolved via `resolve_component_access`); `.build()` returns `Ok`.
2. `conflict_probe_loads_alone_and_is_a_valid_mod` — `ServerModHost::discover_and_load` against a `mods_dir` containing only `conflict_probe`; one `ModLoadDiagnostic` with `outcome: Loaded`.
3. `the_conflict_demo_manifest_variant_and_conflict_probe_form_a_rejected_cycle` — construct `HookOrderInput`s directly from the two mods' *parsed* manifests (`example_ores`'s conflict-demo variant, string-edited per Context; `conflict_probe`'s own real one — `rc_mod_api::parse_manifest`, no dylib load needed for this specific assertion, mirroring M8-B03's own test 4 precedent); `resolve_hook_order(DomainGroup::Lighting, &inputs)` returns `Err(ModOrderingError::Cycle { group: Lighting, hooks })` where `hooks` (order-independent, `HashSet` comparison) contains exactly `example_ores:pulse_survey` and `conflict_probe:counter_tick` — the milestone's own acceptance criterion 1, closed.
4. `rejection_happens_before_any_register_mod_system_call_is_ever_made` — a code-level assertion via test structure, not a runtime check (mirroring M8-B03's own test 4's own documented convention): this test never calls `register_mod_system`/`build()` for the conflicting pair at all, only `resolve_hook_order` directly — the comment block above test 3 states this explicitly.

### `crates/scheduler/tests/mod_reference_crash_isolation.rs` (new)

Uses the full harness (Context, "The end-to-end harness").

1. `pulse_survey_runs_normally_without_the_force_panic_env_var` — build+load `example_ores`; no `EXAMPLE_ORES_FORCE_PANIC` set; `RcExecutorBuilder`/`register_mod_system`/`build()`; `spawn_region`; `tick_region` once against `NoopTransport`; assert the returned `TickReport.tick_counter == 1`, and (via `EXAMPLE_ORES_FIXTURE_LOG_PATH`) the log file gained exactly one `"pulse_survey"` line.
2. `force_panic_is_caught_disables_only_example_ores_tick_region_survives` — set `EXAMPLE_ORES_FORCE_PANIC=1`; `tick_region` once; the call returns normally (the test's own thread never panics); the log file gained **no** new line (the panic fires before the log write, Deliverables' own body ordering); a second `tick_region` call also returns normally with no new log line (the mod's shim is now a permanent no-op, M8-B03's own already-proven disable-path).
3. `a_sibling_region_with_only_a_native_system_keeps_ticking_across_25_calls` — two regions, one carrying `pulse_survey`'s shim (forced to panic on its first tick, `EXAMPLE_ORES_FORCE_PANIC=1` for the whole test), the other carrying only an ordinary always-`Ok` native system incrementing a shared counter; drive both regions through 25 `tick_region` calls each; assert the sibling region's own counter reaches exactly 25, every one of the 50 total `tick_region` calls (both regions) completes in under 50 ms (`std::time::Instant`-measured, no `sleep`), and neither region's own `tick_counter` ever skips or repeats a value — the milestone's own acceptance criterion 2, closed end to end with real content.
4. `disabling_example_ores_never_corrupts_the_engine_exported_test_components` — after test 3's run, a plain `Query<&PulseFlag>`/`Query<&PulseCount>` over the panicking region's own `World` (obtained via a test-only accessor on `RegionState`, or by constructing a fresh region and asserting it still spawns cleanly with both components registered) confirms both remain valid, registered components — a disabled mod's shim never corrupted the export table itself (M8-B03's own "never touches the startup conflict graph" guarantee, reused).

### `crates/scheduler/tests/mod_reference_hook_dispatch.rs` (new)

1. `client_render_hook_is_recorded_headlessly` — `ClientModHost::discover_and_load` against `example_ores`; `call_on_client_registry_build` returns `Ran(Ok(()))`; the passed `ClientRegistryBuildContext.registrations() == [ClientRegistration::BlockRenderer(Identifier::parse("example_ores:pulse_crystal").unwrap())]` — the milestone's own acceptance criterion 3's client-side half, "registered + headless-verified only," PLAN-D2 restated as this test's own explicit scope boundary (no further assertion about rendering exists or is attempted).
2. `client_init_proves_isomorphism_via_the_logged_shared_result` — `call_on_client_init` with `EXAMPLE_ORES_FIXTURE_LOG_PATH` set; the log file's one line reads exactly `"client_init lit=true param=1"` — matching `next_pulse_event(false)`'s own known value, cross-checked directly against `example_ores_shared::next_pulse_event(false)` called from the test itself (this crate does not depend on `example-ores-shared` as a normal dependency — the comparison value is a literal `(true, 1u8)` restated in the assertion, with a comment citing `mods/example-ores/shared/tests/next_pulse_event.rs` test 1 as the source of truth for that literal).
3. `registry_build_fires_with_correct_data_through_the_real_host` — `ServerModHost::call_on_registry_build` against a fresh `RegistryBuildContext::new(0,0)`; `Ran(Ok(()))`; `into_recorded()` matches `registry_build_recording.rs`'s own already-proven shape (2 blocks, 1 item, 1 component, 2 behaviors, 1 channel) — the *same* proof as the mod's own unit test, now reached through the real dylib-loaded path instead of a bare struct construction, closing the gap between "the logic is correct" and "the logic is correct when actually loaded."

### `crates/scheduler/tests/mod_reference_template_conformance.rs` (new)

Reads raw file text under `mods/example-ores/` relative to `CARGO_MANIFEST_DIR` (`../../mods/example-ores`) — no `toml` parsing dependency added (Constraints).

1. `manifest_toml_parses_and_validates` — `rc_mod_api::parse_manifest` then `validate_manifest` against the real, committed `manifest.toml` text; both `Ok`.
2. `workspace_root_declares_exactly_the_three_expected_members` — `Cargo.toml`'s text contains the literal substring `members = ["shared", "server", "client"]`.
3. `server_and_client_both_declare_cdylib_and_rlib` — `server/Cargo.toml` and `client/Cargo.toml`'s text both contain `crate-type = ["cdylib", "rlib"]`.
4. `shared_declares_no_cdylib` — `shared/Cargo.toml`'s text contains no occurrence of `"cdylib"`.
5. `server_and_client_both_depend_on_shared_by_relative_path` — both files' text contains `path = "../shared"`.
6. `manifest_entrypoints_name_shared_server_and_client` — the committed `manifest.toml`'s `[entrypoints]` table, once parsed, has `shared == Some("shared")`, `server == Some("example_ores_server_entry")`, `client == Some("example_ores_client_entry")`.

### `crates/chunk-storage/tests/reference_mod_persistence_roundtrip.rs` (new)

Defines two small, hand-authored mock `BlockStateNames` implementations (`BootAResolver`, `BootBResolver`, Context: "Persistence" section's own two id assignments) local to this test file — no `rc-mod-api` dependency added to `rc-chunk-storage` (Constraints).

1. `direct_bits_before_and_after_matches_ceil_log2` — `ceil_log2(1023) == 10`; `ceil_log2(1025) == 11` — the concrete WORLD-D2 consequence (Context), independent of any container.
2. `pulse_crystal_states_round_trip_through_different_boot_assignments` — a `BlockStateColumn` seeded with a handful of positions set to `BootAResolver`'s own `pulse_crystal[lit=false]`/`[lit=true]` ids (`1023`/`1024`); `ChunkNbtCodec { block_names: &BootAResolver, .. }.to_nbt(...)`; then `ChunkNbtCodec { block_names: &BootBResolver, .. }.from_nbt(...)` on the resulting NBT bytes; assert the reconstructed `BlockStateColumn`'s same positions now hold `BootBResolver`'s own ids (`1030`/`1031`) — the identity that survived is the name, never the number.
3. `a_position_never_touched_by_this_mod_is_unaffected` — a control position holding a synthetic "vanilla" id present in both resolvers under the same number; round-trips to the identical id under both boots, proving the divergence in test 2 is specific to the mod's own ids, not a general resolver artifact.
4. `unresolvable_name_under_the_load_time_resolver_is_a_hard_error` — construct NBT bytes under `BootAResolver`, then attempt `from_nbt` with a *third* resolver that has never heard of `"example_ores:pulse_crystal"` at all; `Err(ChunkNbtError::UnknownBlockStateName(_))` — MOD-D6's own registry-freeze-after-boot discipline extended honestly: a chunk saved with a mod installed cannot silently load under a build that no longer has it.

## Implementation steps

1. **`ModUpdateContext::new`.** Resolve the `stabby::closure` moderate-confidence flag (Deliverables note) against the installed `stabby` 72.1.16 crate first; reconcile M8-B01's own field declarations if needed; then add the constructor exactly as specified (or its reconciled equivalent). Observable: `cargo build -p rc-mod-api --features native-tier` succeeds; M8-B01's own full test suite still passes unmodified.
2. **`mod_host_bridge.rs`.** Implement `native_mod_hook_invoke` exactly as specified. Observable: `cargo build -p rc-scheduler --all-features` succeeds; M8-B03's own full test suite still passes unmodified.
3. **`mods/example-ores/shared/`.** Write `Cargo.toml`, `src/lib.rs`, `tests/next_pulse_event.rs` exactly as specified. Observable: `cargo test --manifest-path mods/example-ores/shared/Cargo.toml` passes.
4. **`mods/example-ores/server/`.** Write `Cargo.toml`, `src/lib.rs` (resolving the `dynptr!` construction moderate-confidence flag against installed `stabby` docs at this point), `tests/pulse_crystal_behavior.rs`, `tests/registry_build_recording.rs`. Observable: `cargo test --manifest-path mods/example-ores/server/Cargo.toml` passes (unit-test level, no dylib loading); `cargo build --manifest-path mods/example-ores/server/Cargo.toml` produces a real `cdylib`.
5. **`mods/example-ores/client/`.** Write `Cargo.toml`, `src/lib.rs` exactly as specified. Observable: `cargo build --manifest-path mods/example-ores/client/Cargo.toml` produces a real `cdylib`.
6. **`mods/example-ores/manifest.toml`.** Write exactly as specified. Observable: `mod_reference_template_conformance.rs` test 1 passes once step 8's harness exists.
7. **`mods/conflict-probe/`.** Write `Cargo.toml`, `manifest.toml`, `src/lib.rs` exactly as specified (mirroring step 4's own resolved `dynptr!` syntax). Observable: `cargo build --manifest-path mods/conflict-probe/Cargo.toml` produces a real `cdylib`.
8. **`crates/scheduler/tests/common/mod_fixture.rs`.** Implement `build_and_package_mod`/`native_binary_sha256`/`NoopTransport` exactly as specified. Observable: compiles; exercised by every remaining step.
9. **`crates/scheduler/tests/mod_reference_conflict_graph.rs`.** Observable: passes in full — closes acceptance criterion 1.
10. **`crates/scheduler/tests/mod_reference_crash_isolation.rs`.** Observable: passes in full — closes acceptance criterion 2.
11. **`crates/scheduler/tests/mod_reference_hook_dispatch.rs`.** Observable: passes in full — closes acceptance criterion 3.
12. **`crates/scheduler/tests/mod_reference_template_conformance.rs`.** Observable: passes in full.
13. **`crates/chunk-storage/tests/reference_mod_persistence_roundtrip.rs`.** Observable: passes in full.
14. **`.gitignore`, `crates/scheduler/Cargo.toml`.** Additive edits exactly as specified. Observable: `git status` shows no untracked `mods/**/target/` noise after a build; `cargo metadata -p rc-scheduler` succeeds.
15. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all exit 0 (the main workspace only; `mods/` is exercised by its own explicit `cargo build`/`cargo test --manifest-path` invocations, step 16).
16. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50), including the two `mods/` manifests' own explicit build/test invocations.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding**, with the same named exception class M8-B01/B02 already established: the two `dynptr!`/`stabby::closure` construction call sites (Deliverables, `server/src/lib.rs`/`client/src/lib.rs`'s entry-factory exports, and `ModUpdateContext::new`'s own field types) are resolved against the installed `stabby` 72.1.16 crate at implementation time, per the moderate-confidence flags stated — no other file, test, or assertion in Acceptance tests may be added, removed, renamed, or weakened by the implementation changeset.

(b) **No new external dependencies beyond the pinned `[workspace.dependencies]` set**, for every crate inside the main workspace (`rc-mod-api`, `rc-scheduler`, `rc-chunk-storage`) — `zip` (already pinned) is `rc-scheduler`'s only new line, dev-only. The two `mods/` packages, being outside the main workspace, pin their own versions directly (`stabby = "72.1.16"`, matching MOD-D3 exactly) — this is not a violation of the main workspace's dependency ceiling, since these packages are never members of it. No `toml`/`semver`/`tempfile` crate is added anywhere (Deliverables/Acceptance tests both note the deliberate simplifications this avoids).

(c) **No Mojang or third-party reimplementation code.** Every mechanism here (the fixture-build technique, the `ModHookInvoke` bridge, the dense-id/palette-bit-width numbers, the persistence round-trip) is derived solely from `06-modding-api.md`'s MOD-D1–D32, this blueprint's own prerequisite blueprints (M8-B01/B02/B03, M3-B01, M2-B01, M2-B04, M0-B05), and this blueprint's own concrete, cited resolutions of what those decisions leave open (ASSET-D18/D19/D30). `pulse_crystal`'s own boolean-property string convention (`"false"`/`"true"`) is this project's own restatement of vanilla's own public, documented convention, not any copied text.

(d) **`unsafe` code is permitted only where `stabby`'s/`libloading`'s own APIs already require it**, reusing exactly the sites M8-B01/B02 already establish (the `#[stabby::export]`-annotated entry-factory functions; no new `unsafe` block is introduced by `mod_host_bridge.rs`, which touches no raw pointer or FFI boundary itself — it only calls `ServerModHost::call_on_tick_hook`, a safe function).

(e) **`crates/server/` is never touched.** This blueprint proves the milestone's own acceptance criteria through its own, new, headless `rc-scheduler`-hosted harness (Context, "The end-to-end harness") — never by wiring `ServerModHost`/`RcExecutorBuilder`/mod loading into `rusty-clanker-server`'s real startup sequence, which remains, honestly, "a future composition-root blueprint's job" (M8-B02/B03's own already-stated deferral, unchanged by this blueprint).

(f) **Scope boundary — do not implement beyond this blueprint's stated Deliverables.** This blueprint does not implement: the WASM tier in any form (native-only, matching the roadmap's own explicit M8-alpha scoping, restated by M8-B02); mod-registered-component ECS resolution (Context, "named, binding limitation," inherited from M8-B03 — a future, not-yet-numbered blueprint's job); a real, wall-clock-paced, multi-region 20 TPS driver (M0-B05's own already-named future work, Context, "Continues at 20 TPS"); a real, per-boot `BlockStateNames` bridge from `rc-mod-host`'s own dense-id allocation into `rc-chunk-storage`'s resolver trait (Context, "Persistence" — this blueprint's own test supplies both resolvers by hand); an actual `cargo generate`-invokable templated repository (MOD-D27's own tooling half, still deferred — this blueprint delivers the concrete, tested crate-tree shape such a template would eventually scaffold, never the `cargo-generate` wiring itself); `rc-mod-test` (MOD-D29, a separate, later blueprint); item obtainability, crafting, or any real client rendering (all explicitly, honestly out of scope at M8, restated throughout Context).

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-mod-api -p rc-scheduler -p rc-chunk-storage --all-features
cargo nextest run -p rc-mod-api -p rc-scheduler -p rc-chunk-storage
cargo test --doc -p rc-mod-api -p rc-scheduler -p rc-chunk-storage
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test

cargo build --manifest-path mods/example-ores/Cargo.toml
cargo test --manifest-path mods/example-ores/Cargo.toml
cargo build --manifest-path mods/conflict-probe/Cargo.toml
```

Expected: every command exits 0. `mod_reference_conflict_graph.rs` (4 cases), `mod_reference_crash_isolation.rs` (4), `mod_reference_hook_dispatch.rs` (3), `mod_reference_template_conformance.rs` (6), `reference_mod_persistence_roundtrip.rs` (4), `pulse_crystal_behavior.rs` (4), `registry_build_recording.rs` (6), `next_pulse_event.rs` (3) — 34 new test cases across both workspaces, all pass, with zero flakiness (no `std::thread::sleep`-based synchronization anywhere in this blueprint's own tests; wall-clock duration is only ever *measured*, never used to gate a wait). CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs, plus the two explicit `mods/` build/test invocations, is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
