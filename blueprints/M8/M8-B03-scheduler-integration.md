# M8-B03 — Mod Systems in RC-Executor: Access Translation, Domain-Group Slotting & Crash-Isolated Dispatch

| Field | Content |
|---|---|
| ID | M8-B03 |
| Milestone | M8 — Mod API Alpha |
| Prerequisites | M8-B01 (`rc-mod-api`'s complete public surface — `Identifier`, `ModId`, `DomainGroup` (5-variant mirror), `TickPriority` (7-variant mirror), `AccessKind`, `ComponentAccessDecl`, `HookDecl`, `HookOrderRef`, `NativeDomainMarker` — every one of these is consumed by this blueprint exactly as M8-B01 fixed it; this blueprint adds nothing to and modifies nothing in `rc-mod-api`); M0-B05 (`rc-scheduler`'s RC-Executor — `RcExecutorBuilder`, `register_system`, `SystemFactory`, `SystemId`, `ExecutorBuildError`, `ComponentAccessSummary`, `compute_waves`, `RegionState`, `RcExecutor`, `Stage` — this blueprint's own new `RcExecutorBuilder` methods are strictly additive extensions of this exact API, restated in full below since M8-B01 itself could only reference it by name); M3-B01 (`rc-mechanics`'s `BlockBehaviorRegistry`/`BlockBehavior`/`UpdateContext` — the *separate*, cheaper mechanism a modded block's per-block tick behavior uses instead of a generic hook this blueprint's own machinery would otherwise have to serve, restated so this blueprint's scope boundary against it is exact, not implemented here; also M3-B01's own `RegionMessageBus`-in-a-system bridge pattern, reused by analogy for this blueprint's own bootstrap-replay problem); M4-B01 (`rc-scheduler`'s `DomainGroup`/`Stage` widening — `EntityAiSelection`/`EntityPhysicsIntegration` replacing the original `AiPhysics`, `RandomTick`/`BlockEntity` added by the prior M3-B06 — the exact, current 8-variant/12-stage shape this blueprint's translation code targets, restated in full since M8-B01 was written against the original 5-variant shape and is stale on this one specific point, corrected here) |
| Implements | MOD-D8/D9/D10/D11/D12 (`06-modding-api.md`, restated in full — the concrete `rc-scheduler`-side mechanism realizing every one of these for the first time); ARCH-D8 (the startup conflict graph — mod systems become first-class, indistinguishable participants); ARCH-D4 (dynamic component registration — the engine-component export table this blueprint adds is the missing "stable name → `ComponentId`" half `ARCH-D4`'s own primitive never supplied); ARCH-D9 (sync points — a mod system's writes flow through the identical mechanism, no new mechanism added); ARCH-D13 (Stage-4 sequential collapse — inherited unconditionally, restated for the mod case); PERF-D46 (`panic = "unwind"` — restated as the binding precondition this blueprint's own disable-path reaction to a mod hook's `Err` return depends on, even though this blueprint itself never calls `catch_unwind`) |
| Crates touched | `rc-scheduler` (`crates/scheduler/`) only |
| Estimated scope | L — a deliberate, cited exception to the ~800-line sizing guideline (`00-blueprint-spec.md`), matching the precedent M8-B01/M6-B07 already established for a blueprint that must resolve several genuinely separate structural gaps at once, none of which is safely splittable without leaving the others without a coherent base to build on: (1) a stable-name → `ComponentId` export mechanism `01`/`06` name the *need* for but never design (Context: "The engine-component export table"); (2) a cross-mod `before`/`after` ordering algorithm, distinct from `compute_waves`'s own access-conflict algorithm, that MOD-D10 names only as resolving "into `ARCH-D8`'s existing `order_tag`" without ever specifying the algorithm itself (Context: "Cross-mod hook ordering"); (3) a live, crash-safe disable mechanism that has to coexist with ARCH-D8's own "conflict graph computed once... reused for every tick... no system list changes mid-run" invariant without violating it; (4) the `DomainGroup`/`Stage` mapping onto a pipeline shape that changed twice (M3-B06, M4-B01) since `06`/M8-B01 were written against the original five-group shape — M8-B01's own `access.rs` doc comment on `DomainGroup` already names this exact conversion as "a future `rc-scheduler` blueprint['s]" job; this is that blueprint. |

## Goal & Done definition

Give `rc-scheduler` the machinery that turns a mod's manifest-declared hook (M8-B01's `HookDecl`/`ComponentAccessDecl`, already parsed and validated by `rc_mod_api::parse_manifest`/`validate_manifest`) into a genuine `RcExecutorBuilder` registration: translating a declared `{name, access, group}` set into a real `bevy_ecs::query::Access<ComponentId>`-backed `ComponentAccessSummary` via a new engine-component name-export table; mapping M8-B01's 5-variant `mod_api::DomainGroup` onto `rc-scheduler`'s current 8-variant `DomainGroup` (post-M4-B01); resolving `before`/`after`/`native:<domain>` ordering declarations, across every mod targeting one domain group, into the linear call sequence that determines each hook's `order_tag`, with an unresolvable cycle rejected at boot with a diagnostic naming every hook still stuck in it; feeding every resulting `ComponentAccessSummary` into `compute_waves` exactly as a native system's is, so a mod system's participation in the startup conflict graph is not merely analogous to a native system's but the literal same code path; giving a mod hook's own runtime failure (a native-tier panic already reduced to an `Err` by a future `rc-mod-host`'s own `catch_unwind`, or a WASM-tier trap) a scheduler-side reaction that disables only that mod, permanently, for the rest of the run, without ever touching the startup-computed conflict graph or wave structure; and giving `exclusive_world_access` mods a real, honestly-costed dispatch path that falls out of `compute_waves`'s own already-proven wildcard-access rule for free.

This blueprint does **not** implement: `rc-mod-host`'s dylib/WASM loading, ABI boundary, or `catch_unwind` wrapping (MOD-D2/D3/D26/D32 — a separate, not-yet-written M8 blueprint, referred to throughout as "a future `rc-mod-host` blueprint"); the actual per-entity data marshaling a real mod hook invocation performs (WIT canonical-ABI lifting/lowering or `stabby`-safe native calls — the same future blueprint's job); block-behavior registration's own dispatch (M3-B01's `BlockBehaviorRegistry` — already a complete, working, *separate* mechanism this blueprint does not touch or duplicate); mod-defined *new* component registration (`RegistryBuildContext::register_component`, M8-B01 — this blueprint's export table covers only pre-existing *engine* components an engine author explicitly opts to expose by name, Context: "The engine-component export table"); the real composition-root integration that calls this blueprint's registration functions in the right sequence for a real, loaded mod set (a future composition-root blueprint's job, extending M6-B07's `build_server_executor`) — this blueprint's own acceptance tests exercise every algorithm and registration path directly, with synthetic mod-shaped test doubles, exactly as M0-B05 exercised its own pipeline with synthetic systems before any real mechanics content existed.

Done when:

- [ ] `cargo build -p rc-scheduler --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-scheduler`.
- [ ] Every pre-existing `rc-scheduler` test (M0-B05's full suite: `compute_waves_conflict_graph.rs`, `access_compatibility.rs`, `registration_validation.rs`, `pipeline_ordering.rs`, `sync_points.rs`, `determinism.rs`) still passes, byte-for-byte unmodified — this blueprint's changes to `registry.rs`/`executor.rs`/`lib.rs` are strictly additive (new methods, new private fields, one new `ExecutorBuildError` variant); no existing public signature changes shape.
- [ ] `cargo run -p xtask -- lint-deps` exits 0 against this blueprint's own expanded `rc-scheduler` dependency set (Context: "Dependency-graph resolution" — adds `rc-mod-api` and `tracing`, both already pinned in `12-workspace-structure.md`'s `[workspace.dependencies]`).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-scheduler` exits 0.
- [ ] `mod_conflict_graph_integration.rs`'s deliberately-conflicting-mod-pair test asserts a hard `Err` at the ordering-resolution stage (never at `build()`, never a panic) naming both hooks by `Identifier`, matching M8's own acceptance criterion 1's "REJECTED at boot with a clear diagnostic."
- [ ] `mod_disable_path.rs` proves a mod hook's `Err` return disables only that mod: the same tick's sibling systems in the same group complete normally, `tick_region` itself returns without panicking, and the next tick's call into the same mod's shim is a genuine no-op — matching M8's own acceptance criterion 2's scheduler-side half (the `rc-mod-host`-side `catch_unwind` half is explicitly out of scope here, Context).
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### What this blueprint is, precisely, and what three sibling mechanisms it is not

06's own hook catalog (restated in full by M8-B01's Context, "hook catalog structure") names four mechanisms a mod may use to touch the engine; this blueprint is the scheduler-side half of exactly **one** of them:

| Mechanism | Owner | This blueprint's relationship |
|---|---|---|
| Generic tick-domain hook (one `ModSystemShim` per declared `[[hooks]]` entry, entering `ARCH-D8`'s conflict graph) | **This blueprint** (`rc-scheduler`) + a future `rc-mod-host` (invocation) | Implemented here, scheduler side, in full |
| Block-behavior registration (`ModBlockBehavior` → `BlockBehaviorRegistry::register_range`/`register_one`) | M3-B01's `BlockBehaviorRegistry`, already complete | Not touched — M8-B01's own Context already explains why this is strictly *cheaper* than a generic hook ("no new conflict-graph entry, no new `[[hooks]]` manifest declaration, Stage 4's existing single-worker sequential collapse already covers it for free") |
| Registry insertion (`register_block`/`register_item`/`register_component`) | A future `rc-mod-host` blueprint, wrapping M4-B01/ARCH-D4 | Not touched — no scheduling concern, pure registry-table insertion |
| Networking (`register_channel`/`on_channel_message`/`on_mod_message`) | A future `rc-mod-host` blueprint, wrapping `RegionMessageBus`/`RegionMessage::ModMessage` (MOD-D14, not yet added to `rc-messaging`'s `RegionMessage` enum by any merged blueprint) | Not touched |

A mod author who only wants a new block's custom tick behavior (M8's own reference-mod scope: "one new block type with custom tick behavior") therefore never reaches this blueprint's machinery at all — that path is M3-B01's, already built. This blueprint exists for the *other* case 06 names: a mod hook that needs to run once per tick inside one of `ARCH-D8`'s five domain groups, reading/writing a manifest-declared component set, independent of any one block type — the mechanism M8's own acceptance criterion 3 ("the hook contract verified via a headless harness proving each hook fires at the correct pipeline point") is written against.

### Dependency-graph resolution: `rc-scheduler` gains `rc-mod-api`

`12-workspace-structure.md`'s WS-D3 rule 4 prose states plainly that "every other crate that supports modding (`rc-mod-host`, `rc-scheduler`, `rc-mechanics`, both binaries) depends *upward* on it [`rc-mod-api`]" — `rc-scheduler` is named explicitly. `12`'s own Dependency Graph mermaid diagram, however, shows only `sched --> modhost` (already present since M0-B01, unused until now), with no direct `sched --> modapi` edge drawn. This is a genuine, narrow gap between `12`'s own prose and its own diagram; this blueprint resolves it the way the text already points: `rc-scheduler` gains a **new**, additive `rc-mod-api` dependency, matching the text's own explicit statement. `12` should be revised to draw the edge at its own next update (not this blueprint's job, mirroring M8-B01's own identical framing for its own WS-D3 rule 4 gap).

**The exact feature set needed, and why it's minimal.** Every `mod_api` type this blueprint consumes — `Identifier`, `ModId`, `DomainGroup`, `TickPriority`, `AccessKind`, `ComponentAccessDecl`, `HookDecl`, `HookOrderRef`, `NativeDomainMarker` — lives in M8-B01's `access.rs`/`capabilities.rs`/`identifier.rs`, all **unconditional** (no `#[cfg(feature = ...)]` gate; M8-B01's own `lib.rs` re-exports them with no feature attribute). None of the `native-tier`-gated surface (`ComponentDescriptorBuilder`, `ModComponentId`, `ModBlockBehavior`, `ServerModEntry`, etc.) is needed here at all — this blueprint never touches a real dylib boundary. `rc-scheduler`'s new dependency line is therefore `rc-mod-api = { path = "../mod-api", default-features = false }` (suppressing `rc-mod-api`'s own `wasm-tier` default feature, which would otherwise pull `wit-bindgen` for no reason this blueprint needs).

`rc-scheduler` also gains `tracing` (already pinned, `12`'s `[workspace.dependencies]`, `tracing = "0.1.44"`), for the loud, per-MOD-D12/D25/D32 disable/exclusive-access warning logging — restated in "Crash isolation and the disable-path" below.

### The widened `DomainGroup`/`Stage` this blueprint targets (post-M4-B01, restated in full)

M8-B01's own `access.rs` describes `mod_api::DomainGroup` as mirroring "`rc_scheduler::DomainGroup` (M0-B05) one-for-one — same five variants" — true when M8-B01 was written, **stale now**. M3-B06 (already committed) widened `rc_scheduler::DomainGroup` from 5 to 7 (adding `RandomTick`/`BlockEntity` for Stages 5/7); M4-B01 (already committed, this blueprint's own prerequisite) widened it again, from 7 to 8, **replacing** the original `AiPhysics` with two new variants, `EntityAiSelection`/`EntityPhysicsIntegration`, and renumbering every `Stage` discriminant from `EntityAiPhysics = 6` onward. The current, binding shape — this blueprint's own restatement, not M8-B01's stale one:

```rust
// rc_scheduler::pipeline (M0-B05, M3-B06, M4-B01 — unmodified by this blueprint)
pub enum Stage {
    PreTickSync = 1, WorldUpdate = 2, NetworkInboundApply = 3, ScheduledBlockTick = 4,
    RandomBlockTick = 5, EntityAiSelection = 6, EntityPhysicsIntegration = 7,
    BlockEntityTick = 8, Lighting = 9, ChunkSnapshot = 10, PostTickFlush = 11,
    NetworkOutboundEncode = 12,
}
pub enum DomainGroup {
    BlockRedstone, EntityAiSelection, EntityPhysicsIntegration, Lighting,
    ChunkSerialize, NetCodec, RandomTick, BlockEntity,
}
// DomainGroup::ALL order / index(): BlockRedstone=0, EntityAiSelection=1,
// EntityPhysicsIntegration=2, Lighting=3, ChunkSerialize=4, NetCodec=5,
// RandomTick=6, BlockEntity=7.
```

`EntityAiSelection` (Stage 6) is dispatched **read-only** — M4-B01's own binding text: it reuses "the identical read-only code path Stage 11 (`NetCodec`)'s own dispatch already calls," so any system registered there has its deferred-command state silently discarded, never applied. `EntityPhysicsIntegration` (Stage 7) is ordinary conflict-graph-batched, deferred dispatch — `AiPhysics`'s original dispatch style, unchanged in kind. `RandomTick`/`BlockEntity` (Stages 5/8) are M3-B06's own additions, outside MOD-D8's original five named groups entirely.

### Translating `mod_api::DomainGroup` into `rc_scheduler::DomainGroup`

MOD-D8 names five groups a mod hook may target: Block/Redstone, AI+Physics, Lighting, Chunk Serialization, Network Encode/Decode — M8-B01's own `mod_api::DomainGroup` enum mirrors exactly these five, unchanged, since M8-B01 is downstream of the planning corpus's original framing, not of M4-B01's later split. This blueprint's own, binding, cited resolution of the one ambiguous case (`AiPhysics`, which the split turned into two):

```rust
pub fn translate_mod_domain_group(group: rc_mod_api::DomainGroup) -> crate::pipeline::DomainGroup {
    match group {
        rc_mod_api::DomainGroup::BlockRedstone   => crate::pipeline::DomainGroup::BlockRedstone,
        rc_mod_api::DomainGroup::AiPhysics       => crate::pipeline::DomainGroup::EntityPhysicsIntegration,
        rc_mod_api::DomainGroup::Lighting        => crate::pipeline::DomainGroup::Lighting,
        rc_mod_api::DomainGroup::ChunkSerialize  => crate::pipeline::DomainGroup::ChunkSerialize,
        rc_mod_api::DomainGroup::NetCodec        => crate::pipeline::DomainGroup::NetCodec,
    }
}
```

**Why `AiPhysics` maps to `EntityPhysicsIntegration`, never `EntityAiSelection`.** MOD-D8's own `ModSystemShim` pseudocode requires a mod hook to "apply... writes through the already-open `FilteredEntityMut` handles" — a mutation capability. `EntityAiSelection` (Stage 6) is *structurally* read-only (M4-B01's own dispatch-level guarantee, not a convention) — a hook registered there could never honor MOD-D8's own "applies returned writes" text, since its writes would be silently discarded exactly as any native Stage-11-style system's already are. `EntityPhysicsIntegration` (Stage 7) is the ordinary, mutation-capable dispatch style `AiPhysics` always had. **Named, binding limitation:** at M8 alpha, a mod cannot register a read-only, Stage-6-style AI-selection hook at all — `mod_api::DomainGroup`'s own 5-variant enum has no distinct case for it (Context: "What this blueprint is, precisely" table already narrows M8's scope away from AI content). A future `mod-api` revision that adds a sixth `AiSelection` variant, mapping onto `Stage::EntityAiSelection`, is the natural extension point; not needed by any M8-alpha acceptance criterion (the reference mod needs a block tick and a client render hook, neither of which is AI-selection-shaped) and not added here.

`RandomTick`/`BlockEntity` are **unreachable** by any mod hook at M8 by construction — `mod_api::DomainGroup` simply has no variant naming them, so `translate_mod_domain_group`'s match is exhaustive over exactly the five groups 06 names, with no fallback arm needed or possible.

### The engine-component export table

MOD-D8's own text: "the mod loader resolves each declared component name to a `ComponentId`... registering **new** mod-defined components via `ARCH-D4`'s `register_component_with_descriptor`." That sentence answers only the case of a component a mod itself is registering. `06`'s own worked manifest example (M8-B01's Context, the `[[capabilities.components]]` block) declares access to `minecraft:block_state` — an **existing engine** component, never registered by any mod, whose `ComponentId` is a `bevy_ecs`-internal, per-`World`, registration-order-assigned integer with no relationship to the string `"minecraft:block_state"` at all. Neither `01` nor `06` names the mechanism that lets a mod's manifest-declared *name* resolve to that integer. This blueprint supplies it — the "stable exported ids" mechanism the task's own framing names, resolved here for the first time.

**Design constraint this table must satisfy.** M0-B05's own `ComponentId`-consistency invariant ("`RcExecutorBuilder::new` takes a `component_bootstrap: fn(&mut World)`... called once against the prototype `World`... and once again, identically, against every region's `World`") is what lets one conflict graph, computed once against a throwaway prototype `World`, stay valid for every real region's own, separately-constructed `World` — soundness rests entirely on every `World` registering the *same components in the same order*. An export table naming *which* of those components a mod may address by string must be computed against that *same* prototype, using the *same* registration call, or its `ComponentId` values would not match what a real region's `World` actually assigns.

**The mechanism.** `RcExecutorBuilder::new`'s `bootstrap` parameter is a raw `fn(&mut World)` — not a boxed closure — specifically because (M3-B01's own already-established precedent, restated here rather than re-derived) a bare function pointer cannot capture per-call state; that is exactly why M3-B01's own `RegionOwnership` had to be inserted separately, per-region, outside `bootstrap`. An export declaration is **not** per-region-varying data (unlike `RegionOwnership`) — every region needs the *identical* export set — so it does not have that problem, but it does still need a place to *record* each resolved `(Identifier, ComponentId)` pair that survives past the single `bootstrap` call that produces it. This blueprint's resolution avoids inventing process-global mutable state (a `OnceLock`/`static`) entirely by keeping the export list on the **builder itself**, not inside `bootstrap`:

```rust
impl RcExecutorBuilder {
    /// Declares that engine component `T` is addressable by mod manifests under the
    /// stable name `name` (Context: "The engine-component export table"). Call order
    /// across multiple `export_component` calls is irrelevant to correctness (each
    /// call's own closure is independently monomorphized, captures nothing, and is
    /// stored — not invoked — until `build()`); call it any number of times, once per
    /// engine type an engine author chooses to expose to mods. Exporting a name twice
    /// is a caller bug (Deliverables: `ExecutorBuildError::DuplicateComponentExport`).
    pub fn export_component<T: bevy_ecs::component::Component>(&mut self, name: rc_mod_api::Identifier) -> &mut Self;
}
```

Internally, `export_component::<T>` pushes `(name, register_fn)` onto a private `Vec`, where `register_fn: fn(&mut bevy_ecs::world::World) -> bevy_ecs::component::ComponentId` is the non-capturing closure `|world| world.register_component::<T>()` — sound as a bare `fn` pointer specifically because it captures nothing; `T` is baked in at monomorphization time, not captured at runtime. `build()` (already, per M0-B05, calling `bootstrap(&mut prototype)` first) then calls every stored `register_fn` against that same `prototype`, in call-declaration order, collecting the results into a `HashMap<Identifier, ComponentId>` — the resolved `EngineComponentExports` table, computed exactly once, carried onto the built `RcExecutor` unchanged for the rest of the process's life (mirroring `compute_waves`'s own "computed once at startup, reused for every tick of every region," ARCH-D8's own phrase, applied here to name resolution instead of conflict detection).

**Keeping every region's `World` consistent.** `RcExecutor::spawn_region` (M0-B05, already calling `bootstrap(&mut world)` for each fresh region `World`) additionally replays the *same* stored `register_fn` list, in the *same* order, against that region's own `World`, immediately after `bootstrap` — never recomputing the resolved `HashMap` (already fixed at `build()` time), only reproducing the *registration act itself* so this region's `World` assigns the identical `ComponentId` values `build()` already resolved, by the identical "same registrations, same order, same result" invariant M0-B05's own bootstrap replay already relies on for every other component.

```rust
pub struct EngineComponentExports {
    by_name: std::collections::HashMap<rc_mod_api::Identifier, bevy_ecs::component::ComponentId>,
}
impl EngineComponentExports {
    pub fn resolve(&self, name: &rc_mod_api::Identifier) -> Option<bevy_ecs::component::ComponentId>;
    /// Diagnostic/test use — every currently-exported name, in export-declaration order.
    pub fn names(&self) -> impl Iterator<Item = &rc_mod_api::Identifier>;
}
```

**Named, binding limitation.** This table covers only components an *engine author* explicitly opts into exporting via `export_component` — never a mod-registered *new* component (Goal & Done definition's own scope line). A mod hook whose `[[capabilities.components]]` entry names a component neither engine-exported nor (in a future blueprint) mod-registered fails to resolve — `ModAccessError::UnresolvedComponentName` (Deliverables) — a documented, honest M8-alpha boundary, not a silent gap: `resolve_component_access` (below) is written generically enough that a future blueprint adding mod-registered-component resolution only needs to supply a richer resolver closure, never a signature change here.

### Resolving declared access into a `ComponentAccessSummary`

MOD-D8's own text: the shim's `component_access()` "is hand-built from the manifest, not derived from `SystemParam` introspection." This is the hand-building step — pure, and, unlike a native system's access (only known once `.initialize()`'d against a real `World`), computable directly from M8-B01's already-validated `ComponentAccessDecl` list plus a name resolver:

```rust
pub fn resolve_component_access(
    hook: &rc_mod_api::Identifier,
    declared: &[rc_mod_api::ComponentAccessDecl],
    resolve_name: impl Fn(&rc_mod_api::Identifier) -> Option<bevy_ecs::component::ComponentId>,
) -> Result<crate::access::ComponentAccessSummary, ModAccessError>;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ModAccessError {
    #[error("hook {hook} declares access to component {name}, which resolves to no known ComponentId (neither an engine-exported name nor a mod-registered one — see this blueprint's Context, 'named, binding limitation')")]
    UnresolvedComponentName { hook: rc_mod_api::Identifier, name: rc_mod_api::Identifier },
    #[error("hook {hook} declares component {name} with both read and write access in the same declaration list — a hook must pick exactly one access kind per component")]
    ConflictingAccessKind { hook: rc_mod_api::Identifier, name: rc_mod_api::Identifier },
}
```

Algorithm: for each declaration, resolve its `name` via `resolve_name` (`Err(UnresolvedComponentName)` on failure); route into a `reads: HashSet` or `writes: HashSet` by `decl.access`, rejecting (`ConflictingAccessKind`) a component named by the *same* hook under both kinds; build `ComponentAccessSummary::new(reads, writes)` (M0-B05's own constructor, unmodified). A `Write` declaration is **not** additionally inserted into `reads` — `ComponentAccessSummary::is_compatible`'s own compatibility rule (M0-B05: "`writes` is pairwise disjoint from the other's `writes` **and** `reads`") already treats a write as conflicting with any read of the same component without needing the write set to also appear in the read set; this mirrors `bevy_ecs`'s own `Query<&mut T>` convention (a system's declared writes and reads are disjoint categories, not overlapping ones).

**Exclusive access bypasses this function entirely.** `exclusive_world_access = true` (MOD-D12) translates directly to `ComponentAccessSummary::wildcard(false, true)` (`writes_all: true`) — never through `resolve_component_access`, and never even inspecting `declared` (Context: "Exclusive access is `compute_waves`'s wildcard rule, for free," below).

### Cross-mod hook ordering: `resolve_hook_order`

MOD-D10's own text: `before`/`after` "resolve into `ARCH-D8`'s existing `order_tag` (declaration-index tie-break) at startup." M0-B05's `order_tag` is *purely* "this call's 0-based index within group" — assigned automatically, in `RcExecutorBuilder::register_system`/`register_mod_system` **call order**, with no mechanism to insert "before" an already-made call. Resolving `before`/`after` into `order_tag` therefore means resolving it into a **call sequence** *before* any registration call is made — a distinct topological sort from `compute_waves`'s own (that one orders by access-conflict; this one orders by declared preference, and a declared preference is never itself a correctness requirement `compute_waves` needs to know about).

**Inputs, restated from M8-B01's `HookDecl` fields this function actually consumes:**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HookOrderInput {
    pub id: rc_mod_api::Identifier,
    pub before: Vec<rc_mod_api::HookOrderRef>,
    pub after: Vec<rc_mod_api::HookOrderRef>,
    /// `Some` only for a `DomainGroup::BlockRedstone` hook (M8-B01's own
    /// `validate_manifest` already rejects a `priority` on any other group) — `None`
    /// elsewhere, reducing this field to a no-op tie-break everywhere but Stage 4.
    pub priority: Option<rc_mod_api::TickPriority>,
}

pub struct ResolvedHookOrder {
    /// Indices into the input `hooks` slice, in the exact sequence `register_mod_system`
    /// must be called — before this group's native systems are registered.
    pub before_native: Vec<usize>,
    /// As above — after this group's native systems are registered. The default
    /// position for any hook with no explicit `native:<domain>` reference at all.
    pub after_native: Vec<usize>,
}

pub fn resolve_hook_order(
    group: rc_mod_api::DomainGroup,
    hooks: &[HookOrderInput],
) -> Result<ResolvedHookOrder, ModOrderingError>;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ModOrderingError {
    #[error("mod hook ordering cycle in domain group {group:?}: {hooks:?} — each hook's before/after constraints (including the implicit native:<domain> default) form a cycle with no valid linear registration order; the server refuses to start with this mod set (MOD-D10)")]
    Cycle { group: rc_mod_api::DomainGroup, hooks: Vec<rc_mod_api::Identifier> },
    #[error("hook {hook} declares a before/after reference to {reference:?}, which does not match any other hook id passed to this call and is not that hook's own group's native:<domain> marker")]
    UnknownOrderingTarget { hook: rc_mod_api::Identifier, reference: String },
    #[error("hook {hook} declares native:{found:?} in its before/after list, but this call resolves ordering for domain group {expected:?} — a hook may reference only its own group's native anchor")]
    NativeDomainMismatch { hook: rc_mod_api::Identifier, expected: rc_mod_api::DomainGroup, found: rc_mod_api::NativeDomainMarker },
}
```

**Algorithm — a second, independent Kahn's-algorithm topological sort, over a different graph than `compute_waves`'s.** Build a directed graph over `hooks.len() + 1` nodes: one per hook, plus one virtual `NATIVE` anchor node representing "every native system already registered into this group" as a single, coarse-grained position (MOD-D10's own `native:<domain>` marker names only the *domain*, never an individual native system — `mod_api::NativeDomainMarker`'s own five variants confirm this is the intended granularity).

1. For each hook `i`, for each `before[j]`: if `HookOrderRef::Hook(id)`, resolve `id` against `hooks[].id` (`Err(UnknownOrderingTarget)` if no match) to target index `t`, add edge `i -> t` ("`i` before `t`" ⇒ `t` depends on `i`); if `HookOrderRef::NativeDomain(marker)`, `Err(NativeDomainMismatch)` unless `marker` corresponds to `group`, else add edge `i -> NATIVE`.
2. Symmetrically for `after[j]`: `Hook(id)` at index `t` adds edge `t -> i`; `NativeDomain` adds edge `NATIVE -> i`.
3. **Default-after-native rule (this blueprint's own binding resolution, not derivable from `06`'s text alone):** for every hook `i` that has *no* edge touching `NATIVE` from step 1/2 at all, add an implicit edge `NATIVE -> i` — making "runs after every native system in this group" the default for any hook that never mentions `native:<domain>`, matching the operationally sensible default that native engine behavior is not silently reordered by an unconstrained mod. Only a hook that *explicitly* declares `before: [native:<domain>]` gets to run earlier.
4. Run Kahn's algorithm exactly as `compute_waves` already does (M0-B05's own algorithm, reused by structural analogy, not by code sharing — this graph's edges mean "must precede," not "conflicts with," so the two algorithms, though both Kahn's-algorithm topological layering, serve entirely different correctness properties and are implemented as two separate functions): repeatedly take every in-degree-0 node not yet processed, in ascending tie-break order, mark processed, decrement successors. Tie-break key: `(priority.map(ordinal).unwrap_or(NORMAL_ORDINAL), original declaration index)` — for a non-`BlockRedstone` group, every `priority` is `None` (M8-B01's own validation already guarantees this), collapsing the key to pure declaration-index order; for `BlockRedstone`, `TickPriority`'s own already-`Ord`-derived ordinal (`ExtremelyHigh` lowest) sorts first, tie-broken by declaration index — MOD-D11's "preserving deterministic FIFO ordering" restated as a concrete tie-break rule.
5. If every node is processed, `NATIVE`'s own final position `k` in the resulting linear order splits it: `before_native = order[0..k]` (hook indices only, `NATIVE` itself excluded from both output lists), `after_native = order[k+1..]`.
6. If Kahn's algorithm terminates with any node still at in-degree > 0, those nodes (excluding `NATIVE`, which is never itself named in a cycle diagnostic — it is a fixed anchor, not an author-controlled hook) are the cycle; `Err(Cycle { group, hooks: <their ids> })`.

**Resolving "two mods claiming conflicting exclusive access" (M8's own acceptance-criterion phrasing), precisely.** An ordinary access conflict — including one where *both* sides are `exclusive_world_access` mods — is **never**, by itself, a rejection: `compute_waves` (unmodified, M0-B05) resolves any such conflict into a legal, serialized wave sequence, exactly as it already does for two conflicting *native* systems. The **only** hard boot-time rejection this blueprint's own mechanism produces is `resolve_hook_order`'s cycle detection, matching MOD-D10's own literal text ("an unresolvable ordering **or access conflict (including a before/after cycle)**" — read as one compound condition whose only concretely-specified failure mode is the cycle case; an ordinary access conflict resolves, it does not reject, exactly as `ARCH-D8`'s own conflict graph has never rejected an ordinary conflict for a native system either). M8's own illustrative phrasing — "two mods claiming conflicting exclusive access" — names the *realistic scenario* this blueprint's own acceptance test constructs to exercise the cycle path (two mutually-`after`-referencing exclusive hooks), not a second, independent rejection rule: exclusivity does not change *whether* a conflict is legal, only how expensive a *legal* one turns out to be (Context, "Exclusive access," below).

### The startup conflict-graph rejection rule, restated exactly against acceptance criterion 1

Putting the two algorithms above together, the complete answer to "what constitutes a rejected conflict vs. a legal serialized conflict":

- **Legal, serialized (never rejected):** any pair of mod hooks — or a mod hook and a native system — in the same `DomainGroup` whose `ComponentAccessSummary`s are incompatible. `compute_waves`, unmodified, places them in different, sequential waves. This is true regardless of whether either declares `exclusive_world_access`.
- **Hard boot-time rejection (`ModOrderingError::Cycle`):** an unresolvable `before`/`after` ordering among hooks (and the implicit/explicit `native:<domain>` anchor) targeting the same group — detected by `resolve_hook_order`, *before* any of the cyclic hooks is ever passed to `register_mod_system`, hence before `build()` (and `compute_waves`) ever runs on them at all.
- **Hard boot-time rejection (`ExecutorBuildError::AmbiguousMutationAuthority`, M0-B05's own, unmodified):** a single hook's own declared write access overlapping its own `structural_writes` — reused verbatim; at M8 alpha every mod-origin registration's `structural_writes` is always empty (Context, "Structural writes: an honest, current-scope limitation," below), so this specific variant can never actually fire for a mod hook today, but the check still runs, uniformly, for forward compatibility.

Acceptance criterion 1's "a second, deliberately conflicting test mod REJECTED at boot with a clear diagnostic" is realized by this blueprint as an ordering-cycle test (Acceptance tests, `mod_conflict_graph_integration.rs`) — the diagnostic names the domain group and every hook still stuck in the cycle, by `Identifier`, satisfying "which mods, which components, which rule violated" (the "which components" element is subsumed: an ordering cycle is a *hook*-identity conflict, not a *component*-identity one, and the diagnostic says so by construction — `ModOrderingError::Cycle` carries no component field because none is relevant to this rejection class).

### Structural writes: an honest, current-scope limitation

`register_system`'s own `structural_writes: Vec<ComponentId>` parameter (M0-B05) exists because a native system may perform `Commands`-based structural mutation invisible to its own `Access` set. M8-B01's `ComponentAccessDecl` schema has **no** field distinguishing "structural" access from ordinary `Read`/`Write` at all, and its `TickHookContext`/`ModTickInvocationCtx`-adjacent surface names no entity-spawn/despawn capability for a generic tick hook. This blueprint's `register_mod_system` (Deliverables) nonetheless keeps an explicit `structural_writes: Vec<ComponentId>` parameter, mirroring `register_system`'s own — every real M8-alpha caller (including this blueprint's own test suite) always passes `Vec::new()`, since nothing in today's manifest schema can populate it with anything else, but the parameter's presence means the *identical* `AmbiguousMutationAuthority` validation (Context, "The startup conflict-graph rejection rule") already applies uniformly to a mod registration with no separate code path, and a future manifest-schema revision adding a structural access kind — together with a future `rc-mod-host` blueprint wiring a real `Commands`-shaped capability into `TickHookContext` — only needs to start passing a non-empty `Vec` through this already-present parameter, never a signature change here.

### `ModHookInvoke`: the deliberately generic invocation boundary

This blueprint does not implement, and cannot implement, the actual "marshal declared-access-scoped entity data into a WIT call / a `stabby`-safe native call" step — that is entity-marshaling logic that needs real mod content (a real WIT world, a real compiled dylib) to exist against, squarely a future `rc-mod-host` blueprint's job (Goal & Done definition). What this blueprint *does* own is the **slot** that logic plugs into, reusing a mechanism M0-B05 already built and proved sound rather than inventing a new unsafe-access story:

```rust
/// Bundles exactly what a mod hook invocation needs and nothing engine-internal
/// beyond it (Context: "ModHookInvoke"). `world` is the *same* kind of raw,
/// access-scoped cell M0-B05's own `executor.rs` already uses for a multi-member
/// wave's concurrent native dispatch (Implementation step 6 there) — a mod system
/// participates in the identical wave mechanism, so it needs, and is given, the
/// identical capability, backed by the identical soundness argument
/// (`compute_waves`'s compatibility proof), never a new one.
pub struct ModTickInvocationCtx<'w> {
    pub world: bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell<'w>,
    pub access: bevy_ecs::query::Access<bevy_ecs::component::ComponentId>,
    pub current_tick: u64,
}

/// One resolved hook's invocation callback, owned and constructed by a future
/// `rc-mod-host` blueprint — this crate calls it, never defines what it does inside.
/// **Binding contract on the implementer (restated from MOD-D32):** this closure must
/// never let a native-tier panic, or an unrecovered WASM-tier trap, escape as a Rust
/// panic — `rc-mod-host`'s own `catch_unwind`-at-the-FFI-boundary (MOD-D32's literal
/// text) is what reduces either failure mode to this `Result`'s `Err` arm *before*
/// this closure returns. `ModSystemShim` (Deliverables) never itself calls
/// `catch_unwind` around this closure (Constraints (d)) — it only reacts to `Err`.
pub type ModHookInvoke = dyn Fn(ModTickInvocationCtx<'_>) -> Result<(), ModHookFailure> + Send + Sync;

#[derive(Debug, Clone)]
pub struct ModHookFailure {
    pub reason: String,
}
```

`UnsafeWorldCell`'s exposure here is a native-tier-shaped capability in practice: a future WASM-tier `ModHookInvoke` implementation never touches `ctx.world` at all (its own host functions, called from inside the WASM guest, already operate on already-marshaled, pre-copied data — the canonical-ABI boundary M8-B01's `wit/rc-mod-api.wit` already fixes — never a raw `World` pointer, which WASM's own sandboxing model could not accept in the first place). Only a native-tier implementation would dereference `ctx.world`, and only within `ctx.access`'s declared bounds — honesty-based, restated from MOD-D9, unchanged by this blueprint.

### `ModSystemShim` and the per-region instantiation model

```rust
/// One mod hook's engine-side dispatch handle — the concrete, private realization
/// of MOD-D8's `ModSystemShim` pseudocode. Never publicly constructed; produced only
/// by `RcExecutorBuilder::register_mod_system`'s internal `SystemFactory` closure,
/// one fresh instance per region (Context: "per-region instantiation"), sharing the
/// same `disabled`/`invoke` handles across every one of those instances.
struct ModSystemShim {
    mod_id: rc_mod_api::ModId,
    hook_id: rc_mod_api::Identifier,
    access: bevy_ecs::query::Access<bevy_ecs::component::ComponentId>,
    disabled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    invoke: std::sync::Arc<ModHookInvoke>,
}
```

`register_mod_system` (Deliverables) stores a `Registration::Mod` variant carrying `mod_id`, `hook_id`, the raw `declared_access: Vec<ComponentAccessDecl>` (resolved into `access` only at `build()` time, exactly when a native registration's own access is first extracted — Context, "Resolving declared access," is a `build()`-time step for the identical reason M0-B05's own `from_bevy_access` is), `exclusive_world_access`, `disabled`, and `invoke`. At `build()`, once `access` is resolved (or set to `wildcard(false, true)` for an exclusive hook), the SAME internal machinery that already turns a native `Registration` into one `CompiledSystem` (M0-B05's `executor.rs`, **unmodified by this blueprint** — see "Why `executor.rs`'s dispatch logic needs zero changes," below) does so for a mod `Registration` too: the `factory: SystemFactory` field it needs is produced here, once, as a trivial, non-capturing-in-spirit (cheaply `Arc`-cloning) closure:

```rust
// inside registry.rs's build(), for one Registration::Mod entry
let factory: SystemFactory = {
    let (mod_id, hook_id, disabled, invoke, access) = (/* cloned/moved */);
    Box::new(move || Box::new(ModSystemShim {
        mod_id: mod_id.clone(), hook_id: hook_id.clone(),
        access: access.clone(), disabled: std::sync::Arc::clone(&disabled),
        invoke: std::sync::Arc::clone(&invoke),
    }) as Box<dyn bevy_ecs::system::System<In = (), Out = ()>>)
};
```

**Per-region sharing, and why it is correct.** `SystemFactory` (M0-B05) is called once per region at `spawn_region` time, giving each region its own `ModSystemShim` *value* — but every one of those values shares the *same* `Arc<AtomicBool>` (`disabled`) and `Arc<ModHookInvoke>` (`invoke`) the registration call was given. This is a deliberate departure from a native system's own per-region isolation, justified directly by MOD-D25/D32's own text: a faulted mod is "auto-disabled... for **the remainder of the run**" — process-wide, not region-scoped. A single shared `Arc<AtomicBool>`, flipped once, is what makes that literally true: the very next tick, in *every* region that has this mod's shim instantiated — including a region spawned by an ARCH-D6 split or merge *after* the disable event, whose own fresh `ModSystemShim` is constructed from the *same* stored `Registration::Mod` (hence the *same* `Arc`) — observes `disabled == true` and short-circuits, with no additional coordination needed at merge/split time at all. **Caller obligation, stated plainly:** whoever orchestrates loading one mod's several hooks (across possibly several domain groups) must construct exactly *one* `Arc<AtomicBool>` per mod and pass a clone of that same `Arc` to every `register_mod_system` call for that mod's every hook — never a fresh `Arc` per hook — or a fault in one hook would fail to disable the mod's *other* hooks, contradicting MOD-D25/D32's "the offending mod," not "the offending hook." This blueprint's own tests construct and share the `Arc` this way; enforcing the discipline mechanically (so a caller cannot pass mismatched `Arc`s by mistake) is left to whichever future blueprint owns the real multi-hook mod-loading orchestration, flagged here rather than silently assumed.

**Why `executor.rs`'s dispatch logic needs zero changes.** `RcExecutor::tick_region`'s existing wave dispatch (M0-B05) operates purely on `CompiledSystem { factory, access, structural_writes }` values inside a `CompiledGroup` — it has no notion of "native" or "mod origin" at all, and never needed one: a `ModSystemShim`, once wrapped as a `Box<dyn System<In=(),Out=()>>` by its own factory closure, is indistinguishable, from `tick_region`'s point of view, from any native system. This is the literal mechanism behind acceptance criterion 1's "participates correctly in ARCH-D8's startup conflict-graph check" — not an analogy this blueprint draws, but the actual, singular code path both kinds of system pass through. The **only** change `executor.rs` needs is in `RcExecutor::spawn_region`'s construction sequence (Deliverables) — replaying the export list, Context above — and in `RcExecutor`'s own struct gaining the `exports`/`component_exports` fields that construction sequence reads.

### Crash isolation and the disable-path — the sync-point safety argument

`ModSystemShim`'s own `run`/`run_unsafe` body (bevy_ecs API points to verify, below) begins with the disable check, before touching `World` at all:

1. `if self.disabled.load(Acquire) { return; }` — a disabled mod's system is a genuine no-op: it never constructs `ModTickInvocationCtx`, never calls `invoke`, never reads or writes anything.
2. Otherwise, construct `ModTickInvocationCtx { world, access: self.access.clone(), current_tick }` from whatever `World`/`UnsafeWorldCell` access this call's own dispatch path already holds (the safe single-member-wave `.run()` path, or the unsafe multi-member-wave `run_unsafe` path — M0-B05's own two existing dispatch shapes, unmodified, `ModSystemShim` simply receives whichever one its own wave membership determines, exactly as a native system does).
3. Call `(self.invoke)(ctx)`.
4. On `Err(failure)`: `self.disabled.store(true, Release)`; `tracing::warn!(mod_id = %self.mod_id, hook = %self.hook_id, reason = %failure.reason, "mod hook failed; disabling mod for remainder of run (MOD-D25/D32)")`; return normally — **no panic, no propagation**.
5. On `Ok(())`: return normally (writes were already applied through `ctx.world`, honesty-bound by `access`, inside step 3's call — `ModSystemShim` performs no separate "apply" step, mirroring MOD-D8's own "applies returned writes through the already-open `FilteredEntityMut` handles," which is exactly what a native-tier `invoke` implementation's own use of `ctx.world` constitutes).

**Why this never touches the startup conflict graph.** The disable flag is checked *inside* `run`, after dispatch has already placed this system in its wave — `compute_waves`'s own output (which wave, alongside which siblings) is fixed at `build()` time and never revisited. A disabled mod's system still occupies its original wave slot; any *other* system that genuinely conflicted with it (and was therefore forced into a later wave) still waits for it — but now waits for a call that returns in nanoseconds, not for the mod's real (and, by definition of this whole path, faulty) work. **Restated honestly, as the corpus's own convention demands:** this mechanism does not shrink or restructure the conflict graph; it only makes a disabled system's own execution cost negligible. A system that could already run *concurrently* with the disabled one (disjoint/compatible access, same wave) is entirely unaffected either way.

**What is explicitly out of scope here, restated from Goal & Done definition.** The actual `catch_unwind` call — MOD-D32's own literal text, "`rc-mod-host` wraps every native-mod hook invocation in `catch_unwind` at the FFI boundary" — belongs entirely to a future `rc-mod-host` blueprint's own `ModHookInvoke` implementation, never to `ModSystemShim`. `panic = "unwind"` (PERF-D46, `12`'s `[profile.release]`) is the binding, already-fixed precondition that makes such a `catch_unwind` call meaningful at all; this blueprint depends on that precondition being upheld but does not itself call `catch_unwind` anywhere, and Constraints (d) forbids adding one.

### Exclusive access is `compute_waves`'s wildcard rule, for free

MOD-D12's own text: "RC-Executor wraps it in a full-drain barrier... the same mechanism `ARCH-D8` already uses *between* groups, applied *within* one group for this single system." M0-B05's own `compute_waves` doc comment already proves the mechanism this sentence describes exists, unmodified, today: "a system declaring `writes_all`/`reads_all` is... always placed alone in its own wave — a proven consequence of the compatibility rule, not a special case the algorithm needs to implement separately." Mapping `exclusive_world_access = true` to `ComponentAccessSummary::wildcard(false, true)` (Context, "Resolving declared access") is therefore the *entire* implementation of MOD-D12's escape hatch — no new dispatch code, no new barrier primitive. Because a lone-member wave is always dispatched via the **safe** single-member `.run()` path (M0-B05's own Implementation step 6: `unsafe` is reserved for multi-member waves only), an exclusive mod system genuinely receives full, safe, unrestricted `&mut World` access for the duration of its own call — a real full-drain barrier, not a metaphor for one, falling directly out of the wave-count-1 case.

**The cost, restated honestly (MOD-D12's own demand).** Every *other* system in the same `DomainGroup` that could otherwise have run concurrently with this one (disjoint or read/read access) is instead forced to wait for the exclusive system's *entire* invocation — including whatever real (possibly WASM/dylib-crossing) work `invoke` performs — before its own wave can begin, since wave `k+1` never starts until every member of wave `k` has finished (M0-B05's own dispatch discipline, unmodified). This is a genuine, per-tick throughput cost paid by every sibling in that group, not merely by the exclusive mod itself; MOD-D12's own "logs a warning... emits a per-mod duration metric" is this blueprint's own loud, `tracing::warn!`-based acknowledgment of exactly that cost, on every invocation, not only a faulting one (Deliverables, `ModSystemShim`'s `run` body, step 2.5). **Named, deferred integration:** the "dedicated per-mod duration metric" half of MOD-D12 is not implemented by this blueprint — it belongs to whichever future blueprint wires mod-system dispatch into `M6-B02`'s already-existing `MetricsRegistry`/`region_tagged_task`/`measure_inline` mechanism (a real, already-built integration point this blueprint does not depend on and does not extend, since M6-B02 is not among this blueprint's prerequisites).

### Cluster/location transparency — what a mod system may and may not see

Restated exactly, adding no new mechanism (mirroring `06`'s own "Cluster Compatibility" section's framing): a `ModSystemShim`'s `ModTickInvocationCtx.world` is the *same* `UnsafeWorldCell` a native system in the same wave would receive — scoped to exactly one region's own `bevy_ecs::World`, honesty-bound to `access`'s declared component set, for the duration of exactly one call. There is no mechanism anywhere in this blueprint's Deliverables through which a mod hook obtains a reference into any *other* region's `World`, a `NodeId`, or any host-identity value — `ModTickInvocationCtx` carries none, `ModSystemShim` carries none, and neither ever will, by the same "never exposed to mod code anywhere in the API surface" discipline MOD-D15 already states for the whole mod-facing surface. Cross-partition effects (MOD-D14's `ModMessage`) do not flow through this blueprint's mechanism at all — `RegionMessage::ModMessage` does not yet exist in `rc-messaging`'s enum (no merged blueprint has added it), and even once it does, a mod's own `send-mod-message` call is a future `rc-mod-host`-owned host function, entirely outside `ModSystemShim`'s own dispatch, exactly as MOD-D16 requires ("fire-and-forget... never a blocking call... no method... returns a value obtained by waiting on another partition" — `ModSystemShim::run` never awaits anything; it is ordinary synchronous `RcWorkerPool` work, indistinguishable in this respect from any native Stage system).

### Determinism rules for mod systems — engine-enforced vs. documented-only

The four determinism properties M0-B05 already proves (Context there: "Determinism guarantee, restated as four testable properties" — concurrency safety, deterministic apply order, stage ordering, Stage-4 sequential+inline) apply to a `ModSystemShim` with **zero** special-casing, because it is dispatched through the identical mechanism: its wave membership is fixed by the identical `compute_waves` algorithm over the identical `ComponentAccessSummary` shape, its own deferred-command state (empty, at M8 alpha, Context "Structural writes") is applied at the identical sync points, and Stage 4 membership inherits the identical single-worker collapse (ARCH-D13, MOD-D11). This is **engine-enforced**, mechanically, for every mod system exactly as for every native one — no honesty required for *this* part.

What remains **honesty-based, documented-only** (MOD-D9's own asymmetry, restated, unchanged by this blueprint): whether a mod hook's own declared access set is *true* is never checked at the native tier (no runtime aliasing check exists in release builds, `01`'s own already-accepted caveat, extended here to mods verbatim) — a native-tier `ModHookInvoke` implementation that reads or writes a component outside `ctx.access`'s declared bounds through `ctx.world`'s raw `UnsafeWorldCell` corrupts engine state exactly as a lying native system would, with `ModSystemShim` powerless to detect it. A WASM-tier implementation, by contrast, is genuinely incapable of this by construction (`ctx.world` is simply never given to it — Context, "ModHookInvoke"). Separately, and applying to **both** tiers equally: a mod author's own hook body must not introduce its own nondeterminism (wall-clock reads, unseeded randomness, HashMap iteration order leaking into observable state) for that hook's *own* output to be reproducible — the engine enforces nothing here beyond what it already fails to enforce for native mechanics code with the identical obligation; this blueprint neither adds nor removes any check in this category.

### `bevy_ecs` 0.19.1 API points to verify at implementation time

Extending M0-B05's own identically-framed list (its five points, unmodified, apply to `ModSystemShim`'s own `System` impl exactly as to any other), this blueprint adds one further point specific to its own new surface:

6. `World::register_component::<T>() -> ComponentId` (used by `export_component`'s stored `register_fn`) and, separately, whether a `bevy_ecs` 0.19.1 **resource** (as opposed to a per-entity component) participates in the *same* `ComponentId`/`Access<ComponentId>` space `compute_waves`/`ComponentAccessSummary` already operate over, or a disjoint one — needed only if a future caller wants to `export_component` a `Resource` type (e.g. exposing M3-B01's own `CurrentTick` resource by a stable name so a mod hook's declared access can legitimately cover it) rather than an ordinary per-entity `Component`. Nothing in this blueprint's own Deliverables signatures changes if the answer differs; `export_component`'s own bound (`T: bevy_ecs::component::Component`) is deliberately written against the ordinary component trait, not `Resource`, so this point affects only whether a *future* caller can reuse `export_component` unchanged for a resource or needs a sibling `export_resource` — not decided or needed here, since no acceptance test in this blueprint's own suite exports a resource.

## Deliverables

### `crates/scheduler/Cargo.toml` (modify — add two lines; every existing line unchanged)

```toml
[dependencies]
rc-core = { path = "../core" }
rc-messaging = { path = "../messaging" }
rc-mod-host = { path = "../mod-host" }
rc-mod-api = { path = "../mod-api", default-features = false }
bevy_ecs = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
crossbeam-deque = { workspace = true }
crossbeam-utils = { workspace = true }
parking_lot = { workspace = true }
```

### `crates/scheduler/src/lib.rs` (modify — add three module declarations + re-exports; every existing line unchanged)

```rust
mod mod_access;
mod mod_order;
mod mod_system; // ModSystemShim stays private — never re-exported, mirroring executor.rs's own CompiledSystem/CompiledGroup precedent

pub use mod_access::{EngineComponentExports, ModAccessError, resolve_component_access, translate_mod_domain_group};
pub use mod_order::{HookOrderInput, ModOrderingError, ResolvedHookOrder, resolve_hook_order};
pub use mod_system::{ModHookFailure, ModHookInvoke, ModTickInvocationCtx};
```

### `crates/scheduler/src/mod_access.rs` (new)

```rust
use std::collections::HashMap;
use bevy_ecs::component::ComponentId;
use rc_mod_api::{AccessKind, ComponentAccessDecl, DomainGroup as ModDomainGroup, Identifier};

/// Resolved engine-component name -> ComponentId table (Context: "The engine-component
/// export table"). Computed once, at `RcExecutorBuilder::build()` time, against the
/// same prototype `World` `compute_waves`'s own graph is computed from; carried
/// unchanged onto the built `RcExecutor`.
pub struct EngineComponentExports {
    by_name: HashMap<Identifier, ComponentId>,
}

impl EngineComponentExports {
    pub(crate) fn from_resolved(by_name: HashMap<Identifier, ComponentId>) -> Self;
    pub fn resolve(&self, name: &Identifier) -> Option<ComponentId>;
    pub fn names(&self) -> impl Iterator<Item = &Identifier>;
}

/// Translates `mod_api::DomainGroup` (5-variant, MOD-D8's original five) into the
/// current, post-M4-B01 `rc_scheduler::DomainGroup` (8-variant). See Context,
/// "Translating mod_api::DomainGroup" for why `AiPhysics` maps to
/// `EntityPhysicsIntegration`, never `EntityAiSelection`.
pub fn translate_mod_domain_group(group: ModDomainGroup) -> crate::pipeline::DomainGroup;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ModAccessError {
    #[error("hook {hook} declares access to component {name}, which resolves to no known ComponentId (neither an engine-exported name nor a mod-registered one — see this blueprint's Context, 'named, binding limitation')")]
    UnresolvedComponentName { hook: Identifier, name: Identifier },
    #[error("hook {hook} declares component {name} with both read and write access in the same declaration list — a hook must pick exactly one access kind per component")]
    ConflictingAccessKind { hook: Identifier, name: Identifier },
}

/// Hand-builds one hook's `ComponentAccessSummary` directly from its manifest-declared
/// access set (MOD-D8: "not one derived by SystemParam introspection"). `resolve_name`
/// is generic so a future caller with a richer (engine ∪ mod-registered) resolver can
/// reuse this function unchanged (Context).
pub fn resolve_component_access(
    hook: &Identifier,
    declared: &[ComponentAccessDecl],
    resolve_name: impl Fn(&Identifier) -> Option<ComponentId>,
) -> Result<crate::access::ComponentAccessSummary, ModAccessError>;
```

### `crates/scheduler/src/mod_order.rs` (new)

```rust
use rc_mod_api::{DomainGroup as ModDomainGroup, HookOrderRef, Identifier, TickPriority};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HookOrderInput {
    pub id: Identifier,
    pub before: Vec<HookOrderRef>,
    pub after: Vec<HookOrderRef>,
    pub priority: Option<TickPriority>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedHookOrder {
    pub before_native: Vec<usize>,
    pub after_native: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ModOrderingError {
    #[error("mod hook ordering cycle in domain group {group:?}: {hooks:?} — each hook's before/after constraints (including the implicit native:<domain> default) form a cycle with no valid linear registration order; the server refuses to start with this mod set (MOD-D10)")]
    Cycle { group: ModDomainGroup, hooks: Vec<Identifier> },
    #[error("hook {hook} declares a before/after reference to {reference:?}, which does not match any other hook id passed to this call and is not that hook's own group's native:<domain> marker")]
    UnknownOrderingTarget { hook: Identifier, reference: String },
    #[error("hook {hook} declares native:{found:?} in its before/after list, but this call resolves ordering for domain group {expected:?} — a hook may reference only its own group's native anchor")]
    NativeDomainMismatch { hook: Identifier, expected: ModDomainGroup, found: rc_mod_api::NativeDomainMarker },
}

/// Cross-mod before/after + implicit-native-default topological sort (Context:
/// "Cross-mod hook ordering"). Pure — no `RcExecutorBuilder` involvement; the caller
/// (a future mod-loading orchestrator) uses the returned index order to decide the
/// exact `register_mod_system` call sequence for this one domain group, calling
/// `before_native`'s hooks, then this group's own native registration function(s),
/// then `after_native`'s hooks.
pub fn resolve_hook_order(group: ModDomainGroup, hooks: &[HookOrderInput]) -> Result<ResolvedHookOrder, ModOrderingError>;
```

### `crates/scheduler/src/mod_system.rs` (new)

```rust
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use bevy_ecs::component::ComponentId;
use bevy_ecs::query::Access;
use bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell;
use rc_mod_api::{Identifier, ModId};

/// Bundles exactly what one mod hook invocation needs (Context: "ModHookInvoke").
pub struct ModTickInvocationCtx<'w> {
    pub world: UnsafeWorldCell<'w>,
    pub access: Access<ComponentId>,
    pub current_tick: u64,
}

#[derive(Debug, Clone)]
pub struct ModHookFailure {
    pub reason: String,
}

/// Owned and constructed entirely by a future `rc-mod-host` blueprint (Context,
/// binding contract on the implementer: never let a panic or unrecovered trap escape).
pub type ModHookInvoke = dyn Fn(ModTickInvocationCtx<'_>) -> Result<(), ModHookFailure> + Send + Sync;

/// Private — never constructed outside `registry.rs`'s `build()`. The concrete
/// realization of MOD-D8's `ModSystemShim` pseudocode (Context).
pub(crate) struct ModSystemShim {
    pub(crate) mod_id: ModId,
    pub(crate) hook_id: Identifier,
    pub(crate) access: Access<ComponentId>,
    pub(crate) disabled: Arc<AtomicBool>,
    pub(crate) invoke: Arc<ModHookInvoke>,
}

// impl bevy_ecs::system::System for ModSystemShim — Context, "Crash isolation and the
// disable-path," steps 1-5; component_access() returns &self.access; structural
// deferred-command state is always empty (Context, "Structural writes"), so
// apply_deferred/queue_deferred are no-ops. Exact trait-method list per bevy_ecs
// 0.19.1 — moderate confidence, verify per Context's API-points list, point 1-5
// (reused from M0-B05 unmodified) plus point 6 (new).
```

### `crates/scheduler/src/registry.rs` (modify — extends M0-B05's file; every existing public signature unchanged)

```rust
use rc_mod_api::{ComponentAccessDecl, DomainGroup as ModDomainGroup, Identifier, ModId};
use crate::mod_system::ModHookInvoke;

impl RcExecutorBuilder {
    /// New (this blueprint). Context: "The engine-component export table."
    pub fn export_component<T: bevy_ecs::component::Component>(&mut self, name: Identifier) -> &mut Self;

    /// New (this blueprint). Registers one mod hook (MOD-D8's `ModSystemShim`) into
    /// `group` (translated via `translate_mod_domain_group`). `disabled` must be the
    /// *same* `Arc` shared across every `register_mod_system` call for this `mod_id`'s
    /// other hooks (Context: "Per-region sharing, and why it is correct" — a binding
    /// caller obligation, not mechanically enforced by this method). `order_tag` is
    /// assigned identically to `register_system`'s own rule: this call's 0-based index
    /// within `group`'s registration sequence — callers wanting a specific relative
    /// order among mod hooks (and against native systems) must call this method (and
    /// this group's native registration function) in the sequence `resolve_hook_order`
    /// already computed (Context).
    pub fn register_mod_system(
        &mut self,
        mod_id: ModId,
        hook_id: Identifier,
        group: ModDomainGroup,
        declared_access: Vec<ComponentAccessDecl>,
        exclusive_world_access: bool,
        /// Mirrors `register_system`'s own parameter of the same name (Context,
        /// "Structural writes: an honest, current-scope limitation") — every real
        /// M8-alpha caller passes `Vec::new()`; kept as an explicit parameter, not
        /// hardcoded internally, so the identical `AmbiguousMutationAuthority` check
        /// already applies uniformly and no future schema addition needs a signature
        /// change here.
        structural_writes: Vec<bevy_ecs::component::ComponentId>,
        disabled: std::sync::Arc<std::sync::atomic::AtomicBool>,
        invoke: std::sync::Arc<ModHookInvoke>,
    ) -> crate::registry::SystemId;
}

// Registration (private) gains a second variant:
//   enum Registration {
//       Native { factory: SystemFactory, structural_writes: Vec<ComponentId> },
//       Mod {
//           mod_id: ModId, hook_id: Identifier, declared_access: Vec<ComponentAccessDecl>,
//           exclusive_world_access: bool, structural_writes: Vec<ComponentId>,
//           disabled: std::sync::Arc<std::sync::atomic::AtomicBool>,
//           invoke: std::sync::Arc<ModHookInvoke>,
//       },
//   }
// RcExecutorBuilder gains:
//   exports: Vec<(Identifier, fn(&mut bevy_ecs::world::World) -> ComponentId)>,
// (alongside the existing `bootstrap`/`groups` fields, unmodified in shape).

#[derive(Debug, thiserror::Error)]
pub enum ExecutorBuildError {
    // M0-B05's own variant, unmodified:
    #[error("system {system:?} declares mutable Query access to component {component:?} that is also listed in its own structural_writes — a component must have exactly one mutation authority per system, never both (ARCH-D8's Domain Conflict Model)")]
    AmbiguousMutationAuthority { system: SystemId, component: ComponentId },

    // New (this blueprint):
    #[error("engine component export name {0} was declared more than once via export_component")]
    DuplicateComponentExport(Identifier),
    #[error("mod hook {hook} in domain group {group:?}: {source}")]
    ModAccessUnresolved { hook: Identifier, group: ModDomainGroup, #[source] source: crate::mod_access::ModAccessError },
}
```

`build()`'s existing algorithm (M0-B05) gains two additive steps, in this exact position: (1) immediately after `bootstrap(&mut prototype)` and *before* any group's registrations are processed, run every stored `export_fn` against `prototype` in call-declaration order, rejecting a duplicate name (`DuplicateComponentExport`) — collect into `EngineComponentExports`; (2) inside the existing per-group, per-registration loop, a `Registration::Mod` entry computes its `ComponentAccessSummary` via `resolve_component_access`/`wildcard` (Context) instead of `from_bevy_access`, wrapping any `ModAccessError` into `ModAccessUnresolved`, and produces its `factory: SystemFactory` via the trivial `ModSystemShim`-constructing closure (Context) instead of returning the registration's own stored native factory unchanged. Every downstream step (`compute_waves`, the `AmbiguousMutationAuthority` check, `CompiledGroup` construction) is untouched, operating over the uniform `(access, structural_writes, factory)` triple regardless of origin.

### `crates/scheduler/src/executor.rs` (modify — additive only; every existing public signature unchanged)

`RcExecutor` gains two new private fields, populated at `build()` time from the builder's own `exports` list and the resolved `EngineComponentExports`:

```rust
pub struct RcExecutor {
    bootstrap: fn(&mut bevy_ecs::world::World),
    groups: [CompiledGroup; 8], // unchanged shape (M4-B01's own width)
    exports: Vec<(rc_mod_api::Identifier, fn(&mut bevy_ecs::world::World) -> bevy_ecs::component::ComponentId)>,
    component_exports: crate::mod_access::EngineComponentExports,
}

impl RcExecutor {
    /// New (this blueprint) — read-only accessor for diagnostics/tests.
    pub fn component_exports(&self) -> &crate::mod_access::EngineComponentExports;
}
```

`spawn_region`'s existing body (M0-B05) gains exactly one additive line, immediately after its existing `bootstrap(&mut world)` call: replay `self.exports` against `world`, in the identical order used at `build()` time (Context, "Keeping every region's `World` consistent") — the returned `ComponentId`s from this replay are discarded (already known, resolved once, from `component_exports`); the replay's only purpose is reproducing the registration *act* so this region's `World` assigns identical ids. `tick_region`'s dispatch body is **not** modified at all (Context, "Why `executor.rs`'s dispatch logic needs zero changes").

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46, restated exactly, matching every prior blueprint's identical rule):** the test changeset is every file below plus every new/modified `src/*.rs` file from Deliverables with each function body replaced by `todo!()` (field lists, derives, and doc comments stay exactly as specified), plus the `Cargo.toml`/`lib.rs` edits. The implementation changeset fills in bodies only — it must not modify any file under `crates/scheduler/tests/`, must not change any type's field list/derive list/public signature, must not weaken any assertion below, and must not touch any pre-existing M0-B05 test file at all (Constraints (a) restates the one narrow exception class other blueprints have used for a genuinely necessary breaking change — Context above establishes that **no** breaking change to any pre-existing signature is needed here, so that exception does not apply and is not invoked).

A shared `crates/scheduler/tests/common/mod_fixtures.rs` (new, alongside M0-B05's own pre-existing `tests/common/mod.rs`, left untouched) holds helpers reused across this blueprint's own test files:

```rust
// tests/common/mod_fixtures.rs
use rc_mod_api::{AccessKind, ComponentAccessDecl, DomainGroup, HookOrderRef, Identifier, ModId, NativeDomainMarker, TickPriority};

pub fn id(s: &str) -> Identifier { Identifier::parse(s).unwrap() }
pub fn mod_id(s: &str) -> ModId { ModId::new(s).unwrap() }

/// A `ModHookInvoke` that always succeeds and records its own call (entity/component
/// touching is out of scope for this blueprint's own tests — Context's own scope
/// boundary — every test double here is a no-op observer, never a real marshaler).
pub fn recording_invoke(log: std::sync::Arc<std::sync::Mutex<Vec<rc_mod_api::Identifier>>>, hook: Identifier)
    -> std::sync::Arc<rc_scheduler::ModHookInvoke>;

/// A `ModHookInvoke` that always returns `Err` with the given reason — the disable-
/// path's own trigger.
pub fn failing_invoke(reason: &str) -> std::sync::Arc<rc_scheduler::ModHookInvoke>;
```

### `crates/scheduler/tests/mod_domain_group_translation.rs` (pure)

1. `block_redstone_maps_identically` — `translate_mod_domain_group(rc_mod_api::DomainGroup::BlockRedstone) == rc_scheduler::DomainGroup::BlockRedstone`.
2. `ai_physics_maps_to_entity_physics_integration_not_selection` — `translate_mod_domain_group(rc_mod_api::DomainGroup::AiPhysics) == rc_scheduler::DomainGroup::EntityPhysicsIntegration` (the one non-obvious case, Context's own central resolution — asserted explicitly, by name, so a future accidental "simplification" back to `EntityAiSelection` is caught immediately).
3. `lighting_chunkserialize_netcodec_map_identically` — the remaining three variants map to their identically-named `rc_scheduler::DomainGroup` counterparts (three sub-cases, one `assert_eq!` each).

### `crates/scheduler/tests/mod_access_resolution.rs` (pure)

1. `single_read_resolves` — one `ComponentAccessDecl` (`access: Read`) whose name resolves via a trivial closure to a fixed `ComponentId`; the returned `ComponentAccessSummary.reads` contains exactly that id, `writes` is empty.
2. `single_write_resolves_into_writes_only` — as above with `access: Write`; `writes` contains the id, `reads` does **not** (Context: "not additionally inserted into `reads`").
3. `unresolved_name_is_rejected` — a resolver closure returning `None` for every name; `Err(ModAccessError::UnresolvedComponentName { .. })`, naming the correct hook and component `Identifier`.
4. `conflicting_read_and_write_of_same_component_is_rejected` — two decls for the same hook, same component name, one `Read` one `Write`; `Err(ModAccessError::ConflictingAccessKind { .. })` (two sub-cases: `Read` then `Write`, and `Write` then `Read` — both orders rejected).
5. `two_disjoint_components_both_resolve` — two decls naming two different components, both `Write`; the summary's `writes` set has both ids, `reads` empty.
6. `resolved_mod_summary_is_compatible_with_disjoint_native_summary` — a mod-resolved `ComponentAccessSummary` (writes X) and a separately-constructed native-style summary (`ComponentAccessSummary::new` writing Y, X != Y); `is_compatible` (M0-B05's own method, unmodified) returns `true` both directions — proving a mod-origin summary is not treated specially by the compatibility rule at all.
7. `resolved_mod_summary_conflicts_with_overlapping_native_summary` — as above but the native summary also writes X; `is_compatible` returns `false` both directions.

### `crates/scheduler/tests/mod_hook_ordering.rs` (pure)

1. `unconstrained_hooks_default_to_after_native_in_declaration_order` — three hooks, no `before`/`after` at all; `resolve_hook_order` returns `before_native: []`, `after_native: [0, 1, 2]`.
2. `explicit_before_native_places_a_hook_ahead` — hook 0 declares `before: [HookOrderRef::NativeDomain(NativeDomainMarker::Lighting)]` (group `Lighting`), hooks 1/2 unconstrained; `before_native: [0]`, `after_native: [1, 2]`.
3. `explicit_after_reference_between_two_mod_hooks_is_respected` — hook 0 unconstrained, hook 1 declares `after: [HookOrderRef::Hook(hooks[0].id.clone())]`; both land in `after_native`, with 0 preceding 1 in that list (assert the exact `Vec` order, `[0, 1]`, not just "both present").
4. `unresolvable_cycle_between_two_mods_is_rejected` — hook 0 declares `after: [Hook(hook_1_id)]`, hook 1 declares `after: [Hook(hook_0_id)]`; `Err(ModOrderingError::Cycle { group, hooks })` where `hooks` (order-independent — assert via a `HashSet` comparison) contains exactly both ids — the exact scenario Context names as "two mods claiming conflicting exclusive access" made concrete (this test does not need either hook to actually declare `exclusive_world_access` — the cycle-detection mechanism is exercised identically either way, per Context's own "exclusivity does not change *whether* a conflict is legal" resolution; a second, near-identical test below, `mod_conflict_graph_integration.rs`'s own test 4, additionally exercises the full `register_mod_system` path with both hooks *also* exclusive, closing the loop on the milestone's own illustrative phrasing end to end).
5. `native_domain_mismatch_is_rejected` — a hook resolved for group `Lighting` whose `before` list names `HookOrderRef::NativeDomain(NativeDomainMarker::BlockRedstone)`; `Err(ModOrderingError::NativeDomainMismatch { .. })`.
6. `unknown_hook_reference_is_rejected` — a hook's `after` list names `HookOrderRef::Hook(id("nonexistent:hook"))`, no such id present in the input slice; `Err(ModOrderingError::UnknownOrderingTarget { .. })`.
7. `block_redstone_priority_breaks_ties_among_unconstrained_hooks` — group `BlockRedstone`, three unconstrained hooks with `priority` `Normal`, `ExtremelyHigh`, `High` (in that declaration order); `after_native` is `[1, 2, 0]` (index of the `ExtremelyHigh` hook first, then `High`, then `Normal` — ascending priority, overriding raw declaration order).
8. `non_block_redstone_group_ignores_priority_field_ties_break_by_declaration_order` — group `Lighting`, three unconstrained hooks all with `priority: None` (M8-B01's own validation already guarantees this in practice; this test constructs the input directly, bypassing manifest validation, to prove `resolve_hook_order` itself falls back to pure declaration order when priority carries no signal); `after_native == [0, 1, 2]`.
9. `empty_hook_list_returns_empty_order` — `resolve_hook_order(group, &[])` returns `ResolvedHookOrder { before_native: [], after_native: [] }`, `Ok`.

### `crates/scheduler/tests/mod_conflict_graph_integration.rs` (integration — real `RcExecutorBuilder`)

1. `mod_system_enters_the_conflict_graph_like_a_native_one` — `export_component::<common::A>(id("test:a"))`; register one native system (`Query<&mut common::A>`, `DomainGroup::Lighting`) via `register_system`; register one mod hook via `register_mod_system` declaring `Write` access to `test:a`, same translated group. `builder.build()` succeeds; a fresh region's `tick_region` call (both systems instrumented to log their own start/end into a shared, timestamped `Arc<Mutex<Vec<(&str, &str)>>>`) shows the two never overlap (the "active count never exceeds 1" technique, M0-B05's own `pipeline_ordering.rs` test 2, reused verbatim) — proving a real access conflict between a mod and a native system is detected and serialized by the identical mechanism.
2. `disjoint_mod_and_native_systems_run_concurrently` — as above but the mod hook declares access to a *different* exported component (`test:b`, disjoint from `test:a`); using the `std::sync::Barrier` technique (M0-B05's `pipeline_ordering.rs` test 3, reused verbatim), both complete only if genuinely allowed to run concurrently — proving `compute_waves` does not over-conservatively serialize a mod system merely for being mod-origin.
3. `two_mods_with_conflicting_access_and_no_ordering_declaration_are_legally_serialized` — two mod hooks (different `mod_id`s), both declaring `Write` access to the same exported component, no `before`/`after` between them; `builder.build()` succeeds (no rejection — Context's own "legal, serialized" case); the active-count technique confirms they never overlap.
4. `two_exclusive_mods_with_an_ordering_cycle_are_rejected_at_boot` — the milestone's own acceptance-criterion-1 scenario, made concrete end to end: two mod hooks, both `exclusive_world_access: true`, targeting the same group, each declaring `after: [Hook(the other's id)]` — first, `resolve_hook_order` (called directly, as the real orchestration sequence would) returns `Err(ModOrderingError::Cycle { .. })` naming both hook ids; second, this test additionally documents (via a code comment, not a further assertion — `register_mod_system`/`build()` are never reached for a cycle-rejected pair, since the real orchestration sequence checks ordering *before* registering, Context) that `RcExecutorBuilder::build()` is simply never called on this pair at all — the rejection happens strictly earlier, at the ordering-resolution stage, matching Context's own "which stage rejects" resolution precisely.
5. `two_exclusive_mods_with_no_ordering_conflict_build_and_serialize_legally` — as test 4 but neither declares `before`/`after` at all (both default to `after_native`, tie-broken by declaration order, Context); `resolve_hook_order` succeeds, both register via `register_mod_system`, `build()` succeeds, and the active-count technique confirms they never overlap (both wildcard, hence individually isolated into their own waves — `compute_waves`'s own already-proven property, exercised here through two *mod*-origin summaries specifically, not just native ones as M0-B05's own suite already covered).

### `crates/scheduler/tests/mod_per_region_instantiation.rs` (integration)

1. `two_regions_share_the_same_disabled_flag` — register one mod hook with `failing_invoke("boom")` and one shared `Arc<AtomicBool>` `disabled`; `spawn_region` twice (regions A and B); `tick_region` region A once (triggers the failure, sets `disabled`); assert `disabled.load(Relaxed) == true`; `tick_region` region B once (never previously ticked) and assert, via `recording_invoke`-style instrumentation on a *second*, `Ok`-returning sibling hook in the same group, that region B's tick completes normally while the failed hook's own instrumentation (if any were attached) shows zero invocation — the disable state, set only by region A's tick, is already visible to region B without any explicit cross-region signal, because both regions' `ModSystemShim` instances share the one `Arc`.
2. `region_spawned_after_a_disable_event_starts_disabled` — as above, but region B is spawned (`spawn_region`) *after* region A's disabling tick, not before; region B's very first tick already sees the mod's hook as a no-op — simulating the ARCH-D6 split/merge case (Context: "including a region spawned by an ARCH-D6 split or merge after the disable event") without needing real split/merge machinery, exactly as M0-B05's own tests exercise `spawn_region` directly rather than through a full region-lifecycle harness.
3. `disabled_flag_is_per_mod_not_per_hook` — two hooks belonging to the *same* `mod_id`, in two different domain groups, sharing one `Arc<AtomicBool>` (the caller-obligation case, Context); one hook fails; assert the *other* hook (a different, `Ok`-returning invoke, instrumented) is also skipped on its own group's *next* tick after the shared flag flips — proving the shared-`Arc` discipline, once honored by the caller, achieves "the offending mod," not "the offending hook," disablement.
4. `two_different_mods_sharing_no_arc_are_independent` — two hooks with *separate* `Arc<AtomicBool>` instances (two different mods, correctly, per the caller obligation); one fails; assert the other's own `disabled` flag is untouched and its own invoke still fires on the next tick.

### `crates/scheduler/tests/mod_disable_path.rs` (integration)

1. `failing_hook_disables_itself_but_not_a_sibling_in_the_same_group` — two mod hooks (disjoint declared access, so both land in the same wave, or at worst adjacent compatible waves — either is fine for this test) in the same group; one uses `failing_invoke`, the other `recording_invoke`. `tick_region` once: the sibling's invocation is recorded (it ran); the failing one's own `disabled` flag is now `true`; `tick_region` does **not** panic and returns a normal `TickReport`.
2. `disabled_hook_is_a_true_no_op_on_the_next_tick` — same setup, `tick_region` twice; on the second call, the previously-failing hook's `invoke` (swapped, after the first tick, for a `recording_invoke` that would prove it if called) is never called — the disable check short-circuits before `invoke` is ever reached.
3. `disable_does_not_change_wave_membership_of_a_sibling_that_genuinely_conflicted` — the failing hook and a *conflicting* (same-component-write) native system, forced by `compute_waves` into two separate, sequential waves; assert (via the active-count/ordering-log technique) that after the failing hook disables itself, the native system's wave still runs *after* it in every subsequent tick (never promoted into the same wave) — proving the conflict graph itself is untouched by a disable event, exactly as Context's own "never touches the startup conflict graph" claims.
4. `tick_region_completes_and_reports_correct_tick_counter_despite_a_failure` — `tick_region`'s returned `TickReport.tick_counter` is correct (matches `region.tick_counter`'s expected value) on the same tick a failure occurs, proving the failure is fully contained within `ModSystemShim::run` and never escapes to affect `tick_region`'s own bookkeeping.

### `crates/scheduler/tests/mod_exclusive_access.rs` (integration)

1. `exclusive_mod_lands_alone_in_its_own_wave` — one `exclusive_world_access: true` mod hook plus two ordinary, mutually-compatible (disjoint access) native systems in the same group; using the active-count technique across all three, assert the exclusive one's own active-count window never overlaps with *either* other system's, while the two ordinary ones' own windows *do* overlap each other (proving isolation is specific to the exclusive one, not a side effect of the group becoming fully sequential).
2. `exclusive_mod_receives_genuine_mutable_world_access` — the exclusive hook's own `invoke` closure, given `ModTickInvocationCtx`, actually mutates an entity's component value through `ctx.world` (an `unsafe` operation inside the test's own closure, justified by this test's own `// SAFETY:` comment citing the wave-of-one/safe-dispatch guarantee, Context) and the mutation is observed, post-tick, in `region.world` directly — proving the safe single-member dispatch path genuinely grants usable `&mut World`-equivalent access, not merely a well-typed but practically inert handle.
3. `two_exclusive_mods_in_the_same_group_serialize_but_never_error` — two `exclusive_world_access` hooks, no ordering conflict between them (Context, `mod_conflict_graph_integration.rs`'s own test 5, reused by direct reference — this test additionally asserts each one, individually, is alone in its own wave, i.e. two separate single-member waves, not one two-member wave, confirming `wildcard(false,true)` vs `wildcard(false,true)` are mutually incompatible exactly as M0-B05's own `access_compatibility.rs` test 7 (`writes_all_conflicts_with_everything_including_itself`) already proves in isolation, exercised here end to end).

## Implementation steps

1. **`Cargo.toml`.** Add `rc-mod-api`/`tracing` lines exactly as specified. Observable: `cargo metadata -p rc-scheduler` succeeds.
2. **`mod_access.rs`.** Implement `translate_mod_domain_group` (five-arm match, Context). Implement `resolve_component_access` (Context algorithm: resolve each decl, route into `reads`/`writes`, reject conflicting-kind, build `ComponentAccessSummary::new`). Implement `EngineComponentExports::{from_resolved, resolve, names}` (trivial `HashMap` wrapper). Observable: `mod_domain_group_translation.rs` and `mod_access_resolution.rs` pass in full.
3. **`mod_order.rs`.** Implement `resolve_hook_order` per Context's six-step algorithm (build the `hooks.len()+1`-node graph including the implicit default-after-native edge, Kahn's algorithm with the `(priority, index)` tie-break, split around `NATIVE`'s resolved position, cycle detection naming every stuck node). Observable: `mod_hook_ordering.rs` passes in full.
4. **`mod_system.rs`.** Implement `bevy_ecs::system::System for ModSystemShim` (Context's five-step `run`/`run_unsafe` body — disable check, context construction, `invoke` call, `Err`-arm disable+log, `Ok`-arm no-op-beyond-what-`invoke`-already-did; `apply_deferred`/`queue_deferred` as no-ops per "Structural writes"; `component_access()` returns `&self.access`). Verify the six `bevy_ecs` API points (Context) before writing this file's body — none change this blueprint's own signatures, only their bodies' exact method-call spelling. Observable: compiles; no dedicated pure test file (exercised entirely through the integration suites below, matching M0-B05's own precedent for `executor.rs`'s own dispatch internals).
5. **`registry.rs`.** Extend the private `Registration` enum with the `Mod` variant; implement `export_component` (push `(name, |world| world.register_component::<T>())` — a non-capturing closure coercing to the stored `fn` pointer type); implement `register_mod_system` (push a `Registration::Mod`, compute `order_tag` as this group's current registration count, identical rule to `register_system`); extend `build()` with the two additive steps (export resolution against `prototype`, immediately after `bootstrap`; per-`Registration::Mod` access resolution and factory construction inside the existing per-group loop) exactly as Context specifies; add the two new `ExecutorBuildError` variants. Observable: `mod_conflict_graph_integration.rs` passes in full.
6. **`executor.rs`.** Add `exports`/`component_exports` fields to `RcExecutor` (populated at the point `build()` currently constructs the returned value); add the one-line export replay to `spawn_region`, immediately after its existing `bootstrap(&mut world)` call; add the `component_exports()` accessor. Do **not** modify `tick_region`'s dispatch body (Context: "zero changes needed"). Observable: `mod_per_region_instantiation.rs`, `mod_disable_path.rs`, `mod_exclusive_access.rs` all pass in full; every pre-existing M0-B05 test (`compute_waves_conflict_graph.rs` through `determinism.rs`) still passes unmodified.
7. **`lib.rs`.** Add the three module declarations and re-exports exactly as specified. Observable: `cargo build -p rc-scheduler --all-features` succeeds with zero `todo!()` remaining.
8. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps` (against the expanded dependency set), `-- test` — all four exit 0.
9. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding, with no exception this time.** Unlike M4-B01/M6-B07, this blueprint introduces no breaking change to any pre-existing `rc-scheduler` signature (Context, Done-definition's own checklist item) — every file under `crates/scheduler/tests/` from M0-B05 is untouched, and this blueprint's own new test files, once committed in the test-authoring changeset, are never edited, added to, or weakened by the implementation changeset.

(b) **No new external dependencies beyond the pinned set.** Only `rc-mod-api` and `tracing` are added to `rc-scheduler`'s `Cargo.toml`, both already pinned in `12-workspace-structure.md`'s `[workspace.dependencies]` table at the versions given there — neither version is altered, and `rc-mod-api` is pulled with `default-features = false` specifically to avoid transitively pulling `wit-bindgen` for no reason this blueprint needs (Context).

(c) **No Mojang or third-party reimplementation code.** Every algorithm here (the export-table replay discipline, `resolve_component_access`, `resolve_hook_order`'s two-graph topological-sort design, the disable-path's sync-safety argument, the exclusive-access wave-of-one reuse) is derived solely from `06-modding-api.md`'s MOD-D8–D12 and this blueprint's own concrete, cited resolutions of what those decisions leave open, plus the already-committed `01`/M0-B05/M3-B01/M4-B01 mechanisms it reuses by direct reference (ASSET-D18/D19/D30).

(d) **`ModSystemShim` never calls `catch_unwind`, anywhere, under any circumstance.** MOD-D32's own binding text assigns that responsibility entirely to a future `rc-mod-host` blueprint's own `ModHookInvoke` implementation, at the FFI boundary — restated in Context ("Crash isolation," "What is explicitly out of scope here"). Adding a `catch_unwind` call inside `ModSystemShim::run`/`run_unsafe` as a "belt and suspenders" precaution is expressly forbidden by this blueprint: it would (i) duplicate a responsibility MOD-D32 already assigns elsewhere, (ii) silently paper over a future `rc-mod-host` implementation that fails to uphold its own documented contract rather than surfacing that bug loudly, and (iii) require `AssertUnwindSafe`-style reasoning about `UnsafeWorldCell`'s own unwind-safety this blueprint has not undertaken and does not need to.

(e) **`unsafe` code is permitted only where it directly reuses M0-B05's own already-proven invariant, never a new one.** `ModTickInvocationCtx.world`'s `UnsafeWorldCell` exposure is sound only because `ModSystemShim` participates in the identical `compute_waves`-governed wave dispatch every native system does (Context, "ModHookInvoke") — this blueprint's own Deliverables introduce no new `unsafe` block of their own beyond what `mod_system.rs`'s `System` impl needs to satisfy the trait (mirroring `executor.rs`'s own pre-existing `unsafe` usage, M0-B05's Constraints (d), unchanged, unextended in kind). Every such block carries a `// SAFETY:` comment citing `compute_waves`'s compatibility proof by name, exactly as M0-B05's own rule already requires workspace-wide for this class of code.

(f) **Scope boundary — do not implement beyond this blueprint's stated Deliverables.** This blueprint does not implement: any part of `rc-mod-host` (dylib/WASM loading, the ABI boundary, `catch_unwind`, real entity/component marshaling — a separate, not-yet-written M8 blueprint); mod-defined new-component registration or its interaction with the export table (Context, "named, binding limitation" — a future blueprint's job); the real composition-root call sequence that loads a real mod set and calls `resolve_hook_order`/`register_mod_system` in the right order (a future composition-root blueprint extending M6-B07's `build_server_executor`); the per-mod duration metric MOD-D12 also asks for (deferred to a future M6-B02 integration, Context); `RegionMessage::ModMessage` or any cross-partition mod-messaging mechanism (MOD-D14, not yet added to `rc-messaging` by any merged blueprint, and not this blueprint's concern regardless — Context, "Cluster/location transparency"); a sixth `mod_api::DomainGroup` variant for Stage-6 AI-selection hooks (Context, "named, binding limitation" under "Translating mod_api::DomainGroup" — not needed by any M8-alpha acceptance criterion); a per-exported-native-system ordering anchor (`native:<domain>/<system>`, `06`'s MOD-D36) — `resolve_hook_order`'s `NATIVE` node stays the single, coarse, whole-domain-group anchor `mod_api::NativeDomainMarker`'s five variants already fix (Context, "Cross-mod hook ordering"), never a per-system node; MOD-D36's finer-grained anchor is planning-level intent only (06's own Open Questions), deferred to a future `rc-scheduler` blueprint. Do not add placeholder implementations of any of these as a shortcut.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-scheduler --all-features
cargo nextest run -p rc-scheduler
cargo test --doc -p rc-scheduler
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-scheduler` runs every pre-existing M0-B05 test (28 cases, unmodified) plus this blueprint's own 3 (`mod_domain_group_translation.rs`) + 7 (`mod_access_resolution.rs`) + 9 (`mod_hook_ordering.rs`) + 5 (`mod_conflict_graph_integration.rs`) + 4 (`mod_per_region_instantiation.rs`) + 4 (`mod_disable_path.rs`) + 3 (`mod_exclusive_access.rs`) = 35 new cases — 63 total — all pass, with zero flakiness (no test in this blueprint's own suite uses `std::thread::sleep` as a synchronization mechanism, following M0-B05's own established convention exactly). CI (`.github/workflows/ci.yml`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
