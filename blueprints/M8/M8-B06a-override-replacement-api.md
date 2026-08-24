# M8-B06a — Override & Replacement: Behavior-Level, System-Level, Cross-Mod Diagnostics, Parity Surfacing

| Field | Content |
|---|---|
| ID | M8-B06a |
| Milestone | M8 — Mod API Alpha |
| Prerequisites | M8-B01 (`rc-mod-api`'s complete public surface — `Identifier`, `ModId`, `RegistryBuildContext`/`RecordedRegistrations`, `ModBlockBehavior`/`ModUpdateContext`, `DomainGroup`/`ComponentAccessDecl` — restated below exactly as M8-B01 shipped them); M8-B02 (`rc-mod-host`'s `ServerModHost`/`HookOutcome`/crash isolation, restated as context this blueprint's own mechanism must stay compatible with, though this blueprint's own acceptance tests never load a dylib — Context, "Where this blueprint's boundary falls"); M8-B03 (`rc-scheduler`'s `RcExecutorBuilder`/`ComponentAccessSummary`/`compute_waves`/`resolve_hook_order`/`ModSystemShim` — this blueprint's own `resolve_override_order` is a structural sibling of `resolve_hook_order`, restated in full below since M8-B03 could only be referenced by name); M3-B01 (`rc-mechanics`'s `BlockBehaviorRegistry`/`BlockBehavior`/`UpdateContext<'a>`/`BlockWorldAccess`/`BlockEvent` — the exact seam this blueprint's override mechanism extends, restated in full); M4-B06 (`rc-mechanics`'s `FluidBehavior`/`register_fluids`/`FluidBlockRanges` — the canonical override demo target); M0-B05 (`rc-scheduler`'s `SystemFactory`/`SystemId`/`Registration`/`ExecutorBuildError` — the exact registration primitive this blueprint's named-system mechanism extends) |
| Implements | MOD-D33 (the anchor principle, restated and made concrete for the two mechanisms below); MOD-D35 (behavior-level override: `Identifier`-targeted, `Wrap`/`Replace`, call-original); MOD-D36 (native systems gain stable public identifiers, exported explicitly); MOD-D37 (system-level replace/disable riding MOD-D36's export mechanism, stage/domain discipline inherited unconditionally); MOD-D38 (cross-mod override/replace conflict resolution, extending MOD-D10's before/after mechanism verbatim — never a second ordering model); MOD-D34 (parity scoping and discoverability — this blueprint's own scope is the server-side data layer only, Context); ARCH-D8/D13 (conflict graph, Stage-4 sequential collapse — inherited, not modified) |
| Crates touched | `rc-mod-api` (`crates/mod-api/`), `rc-scheduler` (`crates/scheduler/`), `rc-mechanics` (`crates/mechanics/`) |
| Estimated scope | L — a deliberate, cited sizing exception matching M8-B01/B02/B03's own precedent. This is part **a** of a two-part split of the single task-level blueprint `M8-B06` (the task's own "sizing ≤L, split a/b if needed" allowance, exercised here exactly as `blueprints/M5/M8-B12a`–`e` already exercised it for an earlier milestone): part **a** covers the two override tiers (MOD-D35, MOD-D36/D37), their shared conflict-resolution algorithm (MOD-D38), and parity-opt-out reporting (MOD-D34); part **b** (`M8-B06b`) covers the event layer (MOD-D39) and component attachment to vanilla entities (MOD-D41/D42), both structurally independent of this part's own mechanism (06's own text: events sit "underneath — never a substitute for — the override tiers," composing freely but never depending on them). |

## Goal & Done definition

Give mods the two override tiers MOD-D33 requires exist for *every* vanilla-registered behavior or system — never a privileged, mod-unreachable vanilla-only path: MOD-D35's `Identifier`-targeted `Wrap`/`Replace` for block behaviors, proven end to end against `minecraft:water`'s own real vanilla `FluidBehavior` (M4-B06) so that a test mod's replacement is provably the one `BlockBehaviorRegistry::resolve` actually returns; and MOD-D36/D37's named-system export plus disable/replace for `RcExecutor`-scheduled systems, proven inside Stage-4's mandatory sequential collapse (ARCH-D13). Both tiers resolve cross-mod ordering and conflicts through one new, shared algorithm, `resolve_override_order` — a structural sibling of M8-B03's own `resolve_hook_order`, extending MOD-D10's before/after mechanism exactly as MOD-D38 requires, never a second ordering model. A queryable `active_overrides()`/`active_system_overrides()` pair satisfies MOD-D34's discoverability requirement at the data layer, honestly scoped short of the actual `minecraft:brand`/Server List Ping wire surfacing, which remains `02-protocol-networking.md`'s own still-open job (06's own Interfaces section already flags this gap; this blueprint does not close it).

This blueprint does **not** implement: the WASM tier's override surface (deferred identically to every other WASM-tier mechanism in M8 — "a future, not-yet-numbered `rc-mod-host` blueprint," M8-B01/B02/B04's own consistent framing); the real composition-root wiring that reads a *loaded* mod's own override declarations from a real manifest and calls this blueprint's functions in the right sequence (a future composition-root blueprint's job, extending M6-B07 — identical honest deferral to M8-B03's own "the real composition-root integration... is a future composition-root blueprint's job"); the `minecraft:brand`/Server List Ping wire-format change MOD-D34 names (owned by `02-protocol-networking.md`, not yet ratified — Context); the event layer (MOD-D39) and component attachment (MOD-D41/D42), both owned by this task's sibling blueprint, `M8-B06b`.

Done when:

- [ ] `cargo build -p rc-mod-api --all-features`, `cargo build -p rc-scheduler --all-features`, and `cargo build -p rc-mechanics` all succeed with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-mod-api -p rc-scheduler -p rc-mechanics`.
- [ ] Every pre-existing test in all three crates (M8-B01/B02/B03's full suites; M3-B01/M4-B06's full suites) still passes, byte-for-byte unmodified — every change in this blueprint is additive (new methods, new fields, new files); no pre-existing public signature changes shape.
- [ ] `water_override_replace.rs` proves a `Replace`-mode override of `minecraft:water`'s vanilla `FluidBehavior` is the behavior `BlockBehaviorRegistry::resolve` actually returns for every `BlockStateId` in water's own range, and that its custom logic (not vanilla `spread`) is what runs — matching the task's own "a test mod REPLACES water's spread behavior and the engine provably uses it."
- [ ] `mod_behavior_adapter.rs`'s wrap-mode test proves `ModOriginalBlockBehavior`'s closures genuinely invoke whatever behavior previously occupied the target — "original callable."
- [ ] `named_system_override.rs` proves disable/replace for a synthetic system in a chunk-parallel group and for a synthetic system in the `BlockRedstone` group (Stage 4), the latter provably still running single-worker, fully sequential, post-replacement.
- [ ] `override_order_resolution.rs` proves `resolve_override_order`'s full contract: wrap composition, wrap-truncation-by-replace, explicit-order replace-vs-replace resolution, and the double-override diagnostic (`OverrideOrderingError::UnresolvedReplaceConflict`) for two unordered `Replace`s on one target.
- [ ] `unmodded_parity_regression_guard.rs` proves that with zero overrides ever applied, `BlockBehaviorRegistry::resolve(water_state)` returns the exact same `Arc` (`Arc::ptr_eq`) as the pre-this-blueprint baseline, and both `active_overrides()`/`active_system_overrides()` report empty — the scoping proof that this blueprint changes nothing observable for an unmodded server.
- [ ] `cargo run -p xtask -- lint-deps`, `fmt-check`, and `lint` all exit 0.
- [ ] `cargo test --doc -p rc-mod-api -p rc-scheduler -p rc-mechanics` exits 0.
- [ ] CI tier: Tier 1 green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### MOD-D33, the anchor principle, restated exactly

"No special position for vanilla": every vanilla-registered behavior or system dispatches through the identical registry-shaped seam a mod's own registration uses, so a mod may `Wrap`, `Replace`, or `Disable` it through that same seam. Three invariants bind unconditionally, restated as this blueprint's own binding constraints: (1) **stage/domain discipline is inherited, never opted out of** — an override targeting Stage 4 still runs single-worker, fully sequential (ARCH-D13); a replacement targeting a chunk-parallel stage stays chunk-parallel; (2) **the scheduler alone owns parallelism** — every replacement declares its own `Access<ComponentId>` exactly as MOD-D8 requires of a hook and is slotted into the identical conflict graph (`compute_waves`, unmodified); (3) **determinism duties are inherited, not waived** — ARCH-D9's sync-point discipline and MOD-D14–D17's cross-partition rules apply identically to a replacement's own structural mutations and persisted state.

### Where this blueprint's boundary falls

| Piece | Owner |
|---|---|
| Behavior-override chain resolution + installation (native tier) | **This blueprint** (`rc-mechanics`, new `mod_behavior_adapter.rs`) |
| Named-system export + disable + in-place replace (`RcExecutorBuilder`) | **This blueprint** (`rc-scheduler`, extends `registry.rs`/`executor.rs`) |
| Cross-mod ordering algorithm + diagnostics (both mechanisms share it) | **This blueprint** (`rc-scheduler`, extends M8-B03's own `mod_order.rs`) |
| Server-side discoverability query (`active_overrides`/`active_system_overrides`) | **This blueprint** |
| WASM-tier override hosting | Deferred — "a future, not-yet-numbered `rc-mod-host` blueprint," identically to every other WASM-tier mechanism in M8 |
| Real manifest → these-functions wiring for a loaded mod set | Deferred — a future composition-root blueprint, identically to M8-B03's own deferral of `register_mod_system`'s real caller |
| `minecraft:brand`/SLP wire indicator | Deferred — `02-protocol-networking.md`'s own still-open ratification (06's Interfaces section) |
| Per-exported-native-system ordering anchor (`native:<domain>/<system>`, MOD-D36) | Deferred — `resolve_override_order`'s own anchor concept has no equivalent to MOD-D10's `native:<domain>` marker at all (Context, "resolve_override_order," below), and this blueprint does not add one; MOD-D36's finer-grained anchor is planning-level intent only (06's Open Questions), a future `rc-scheduler` blueprint's job alongside M8-B03's own identical deferral |

This blueprint's own acceptance tests exercise every mechanism directly, with real `BlockBehaviorRegistry`/`RcExecutorBuilder` state and, for the flagship demo, a real `FluidBehavior` — but with hand-constructed override *requests* (no manifest TOML, no dylib) standing in for what a future composition-root blueprint will eventually source from a loaded mod's manifest, exactly the same "prove the mechanism now, defer the real wiring" discipline M8-B03's own `mod_conflict_graph_integration.rs` already established for hooks.

### Why no manifest schema change is needed

`register_block_behavior` (M8-B01) is an ordinary `RegistryBuildContext` method call a mod's own `on_registry_build` body makes directly — never manifest-declared, because RegistryBuild is a batch, boot-time, one-shot phase (MOD-D6) with no scheduling concern of its own. Behavior-level overrides (MOD-D35) are the identical shape: this blueprint's new `RegistryBuildContext::override_block_behavior_replace`/`_wrap` methods are ordinary calls a mod's `on_registry_build` makes, recorded (never a live callback, matching M8-B02's own "recording structure" resolution for this exact context type) into `RecordedRegistrations`, and translated into real `BlockBehaviorRegistry` state by a later pass. System-level replace/disable (MOD-D36/D37) needs a declared `Access<ComponentId>` set exactly like a generic hook — but that translation, for hooks, already happens outside the manifest schema too: M8-B03's own `register_mod_system` is called directly by whichever future orchestrator reads a manifest, not derived from a new manifest table this blueprint would have to invent. This blueprint follows the identical shape: `RcExecutorBuilder::register_named_system`/`disable_named_system`/`replace_named_system` are plain Rust API additions a future orchestrator calls; no `manifest.rs` edit is needed or made.

### MOD-D35, restated exactly, and the missing adapter this blueprint builds

MOD-D35: a mod may register an override against an existing block/item/fluid/entity-type's own behavior, targeted by `Identifier` (never a raw state id, which a mod cannot know in advance), in exactly two modes: **`Wrap`** (the new behavior receives a live, callable handle to whatever currently occupies the target — vanilla native logic, vanilla data-driven logic, or an earlier mod's own `Replace`) or **`Replace`** (fully supersedes, no handle). Call-original is tier-transparent (MOD-D35's own text) — for the native tier specifically (the only tier this blueprint implements, matching every other M8 blueprint's native-first scoping), the handle is an ABI-stable closure bundle mirroring `stabby`'s own boundary discipline (MOD-D3), capturing whatever the previous layer's dispatch actually was.

**The gap this blueprint closes.** M8-B01's own `ModUpdateContext` doc comment says it is "constructed host-side by `rc-mod-host` for the duration of exactly one callback" — but M8-B02 (already merged) explicitly confirms `rc-mod-host` *cannot* do this: it has "no `bevy_ecs`/`rc-scheduler`/`rc-mechanics` dependency by design," so it never has a real `rc_mechanics::behavior::UpdateContext<'a>` in scope to bundle closures over. M8-B04's own `ModUpdateContext::new` usage (already merged) only ever backs it with "simple `Cell`/`RefCell`-captured recording closures — no dylib, no `rc-mod-host`... involved at all," used solely for a mod's *own* unit tests — never wired to a real Stage-4 dispatch loop. This blueprint is the first that needs a *real*, engine-wired `ModUpdateContext`, and resolves the gap correctly: the only crate with both a live `UpdateContext<'a>` (M3-B01) and, newly, an `rc-mod-api` native-tier dependency in scope is `rc-mechanics` itself — the natural, minimal-new-mechanism owner, resolved here rather than left open, mirroring M8-B03's own "the engine-component export table... neither `01` nor `06` itself resolves; this blueprint supplies it" precedent.

**`rc-mechanics` gains an `rc-mod-api` dependency** (Deliverables' `Cargo.toml`), mirroring M8-B03's own resolution of the identical WS-D3 rule 4 gap for `rc-scheduler`: `rc-mod-api = { path = "../mod-api", default-features = false, features = ["native-tier"] }`, plus `stabby` directly (needed to name `stabby::dynptr!`/`stabby::boxed::Box`/`stabby::closure::*` types in the adapter's own field declarations). `12-workspace-structure.md`'s own WS-D3 rule 4 prose already names `rc-mechanics` explicitly as an eventual `rc-mod-api` consumer ("every other crate that supports modding... depends *upward* on it"); `12` should be revised to draw this edge at its own next update, matching M8-B01/B03's own identical framing for their own analogous gaps.

### `BlockBehaviorRegistry`'s existing panic-on-overlap behavior is untouched

M3-B01's `register_range`/`register_one` "panics on overlap with an already-registered range" — the ordinary, additive registration path's own collision guard, matching MOD-D35's own text: "the ordinary additive registration path (new mod content) keeps rejecting accidental id/range collisions unchanged." This blueprint adds a **second**, override-only insertion path (`register_named_range` for the *first*, ordinary registration under a stable name, and `apply_override_chain` for the override itself) that never calls, and never changes the behavior of, `register_range`/`register_one`.

### The engine-side name → range table, mirroring MOD-D36's export-table shape

MOD-D35's own rationale names a real, still-open gap: "this document assumes such a registry-shaped seam exists per target kind... flagged as a required exposure" (06's own Interfaces section: "a per-target, registered behavior-dispatch seam for each vanilla block/item/fluid/entity-type behavior `05` specifies... today `05` specifies redstone component behaviors only as their own Stage-4 system"). This blueprint resolves the block/fluid case concretely, the same way M8-B03 resolved the analogous component-name gap: `BlockBehaviorRegistry` gains a private `names: HashMap<Identifier, (BlockStateId, BlockStateId)>` table, populated only by `register_named_range` — an engine author (here, `M4-B06`'s own `register_fluids`, modified additively) opts a range into being override-addressable by calling `register_named_range` instead of the bare `register_range`; every other call site (e.g. a mod's own `register_block_behavior` for a brand-new block, M8-B01) keeps calling the plain, unnamed path and remains override-*unreachable* by construction — the identical "opt it in, or it's unreachable" honesty MOD-D36 already establishes for systems, applied here to behaviors.

**`register_fluids` (M4-B06, modified additively).** Its own body currently calls `registry.register_range(start, end, water_behavior)` and the equivalent for lava, reading the ranges from its own `FluidBlockRanges { water: (BlockStateId, BlockStateId), lava: (BlockStateId, BlockStateId) }` parameter. This blueprint changes exactly those two call sites to `registry.register_named_range(id("minecraft:water"), start, end, water_behavior)` / `id("minecraft:lava")` — `register_named_range`'s own first action is calling `register_range` internally (Deliverables), so the panic-on-overlap collision guard, and every other observable property of the un-named path, is preserved exactly; the only change is that `minecraft:water`/`minecraft:lava` become resolvable by name afterward. No other line of `fluid.rs` changes.

### `resolve_override_order` — MOD-D38's algorithm, a structural sibling of `resolve_hook_order`

MOD-D38: "Two or more mods' `Wrap` declarations against the same override target... resolve via the identical deterministic ordering MOD-D10 already establishes." M8-B03's own `resolve_hook_order` (already merged, unmodified by this blueprint) is that ordering mechanism's first concrete realization — a Kahn's-algorithm topological sort over `before`/`after` edges with a `(priority, declaration-index)` tie-break. This blueprint's `resolve_override_order` reuses the identical algorithmic shape — restated in full below since it is a genuinely new function, not a call into `resolve_hook_order` itself (the two graphs mean different things: `resolve_hook_order`'s edges are "must precede in registration/dispatch order"; `resolve_override_order`'s edges are "must precede in the wrap-chain nesting order," and its own anchor concept — MOD-D10's `native:<domain>` marker — has no equivalent here, since "vanilla's own original" is implicit, always innermost, never an entry of its own to reference).

**Inputs and outputs:**

```rust
/// One mod's own override request against one target (Context: no manifest table
/// backs this — a future orchestrator builds this list from whatever a loaded mod's
/// manifest eventually declares). `load_order_index` is MOD-D31's own resolved
/// dependency load order position — the tie-break key MOD-D40 already uses
/// identically for data-registry replacement, reused here rather than inventing a
/// second tie-break rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverrideOrderInput {
    pub mod_id: rc_mod_api::ModId,
    pub mode: rc_mod_api::OverrideMode,
    pub before: Vec<rc_mod_api::ModId>,
    pub after: Vec<rc_mod_api::ModId>,
    pub load_order_index: u32,
}

/// The resolved outcome for one target (Context: "Resolving the chain," below).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedOverrideChain {
    /// Indices into the input `entries` slice, outermost-first — MOD-D38's own text:
    /// "that linear order *is* the wrap call chain, outermost declared runs first."
    /// Every index here is either a `Wrap` still reachable in normal operation, or
    /// the single winning `Replace` (at most one, always the chain's own last
    /// element when any `Replace` is present at all — Context).
    pub chain: Vec<usize>,
    /// A `Wrap` shadowed by a `Replace` ordered outer of it (MOD-D38: "a `Replace`
    /// truncates any `Wrap` ordered innermost of it") — legal, loud, non-fatal.
    pub truncated: Vec<usize>,
    /// A `Replace` that lost an *explicit* ordering against another `Replace` on the
    /// same target (MOD-D38: "the loser rejected at boot with a diagnostic... never
    /// silently dropped") — distinct from `truncated`: this index is never installed
    /// at all, not merely unreachable.
    pub rejected: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OverrideOrderingError {
    #[error("override ordering cycle on target {target}: {mods:?} — each mod's before/after constraints on this target form a cycle with no valid linear chain; the server refuses to start with this mod set (MOD-D10, extended by MOD-D38)")]
    Cycle { target: rc_mod_api::Identifier, mods: Vec<rc_mod_api::ModId> },
    #[error("mod {mod_id} declares a before/after reference to {reference}, which names no other mod overriding target {target}")]
    UnknownOrderingTarget { target: rc_mod_api::Identifier, mod_id: rc_mod_api::ModId, reference: rc_mod_api::ModId },
    #[error("target {target}: mods {mods:?} each declare Replace with no explicit before/after ordering between them — two full replacements cannot be composed, only ordered (MOD-D38); declare an explicit before/after between these mods for this target, or the server refuses to start")]
    UnresolvedReplaceConflict { target: rc_mod_api::Identifier, mods: Vec<rc_mod_api::ModId> },
}

/// Resolves every mod's own `Wrap`/`Replace` request against one shared `target`
/// into a deterministic chain (Context: "Resolving the chain"). Pure — no
/// `BlockBehaviorRegistry`/`RcExecutorBuilder` involvement; the caller applies the
/// returned chain to whichever real registry owns `target`.
pub fn resolve_override_order(
    target: &rc_mod_api::Identifier,
    entries: &[OverrideOrderInput],
) -> Result<ResolvedOverrideChain, OverrideOrderingError>;
```

**Resolving the chain — the complete algorithm:**

1. Build a directed graph over `entries.len()` nodes only (no anchor node — Context, above). For entry `i`'s `before[j]` naming mod `m`: resolve `m` against `entries[].mod_id` to index `t` (`Err(UnknownOrderingTarget)` if absent); add edge `i -> t`. Symmetrically, `after[j]` naming `m` at index `t` adds edge `t -> i`.
2. Run Kahn's algorithm exactly as `resolve_hook_order` does: repeatedly take every in-degree-0 node not yet processed, tie-broken ascending by `(load_order_index, original declaration index)`, mark processed, decrement successors. Any node still at in-degree > 0 when the algorithm terminates: `Err(Cycle { target, mods: <their mod_ids> })`.
3. Walk the resulting order outermost-to-innermost, tracking `active_replace: Option<usize>` (initially `None`):
   - If `active_replace` is `None`: push the current index onto `chain`; if this entry's `mode` is `Replace`, set `active_replace` to this index.
   - If `active_replace` is `Some(a)` and the current entry's `mode` is `Wrap`: push the current index onto `truncated` (do **not** push onto `chain` — MOD-D38's "a `Replace` truncates any `Wrap` ordered innermost of it").
   - If `active_replace` is `Some(a)` and the current entry (index `b`) is also `Replace`: check whether `a`/`b` share a *direct* edge in the graph built at step 1 (i.e. one names the other's `mod_id` directly in its own `before`/`after` list — a tie-break-only relative position, with no such direct edge, does not count). If yes: `a` is superseded — remove `a` from `chain`, push it onto `rejected`, set `active_replace = Some(b)`, push `b` onto `chain`. If no direct edge exists: `Err(UnresolvedReplaceConflict { target, mods: [entries[a].mod_id, entries[b].mod_id] })`.
4. Return `Ok(ResolvedOverrideChain { chain, truncated, rejected })`.

**Applying to a system-only case (no `Wrap` mode exists for systems, MOD-D37's own text names only disable/replace).** A caller resolving system-level conflicts passes every entry as `mode: OverrideMode::Replace` uniformly (Deliverables, `registry.rs`) — the algorithm above then degenerates exactly correctly: at most one survives in `chain` (its own last element), every other `Replace` is either `rejected` (an explicit loser) or the whole resolution is `Err(UnresolvedReplaceConflict)` if two or more `Replace`s share no direct edge — precisely MOD-D38's own rule, with zero special-casing for the system case.

### `OverrideMode`, shared by both mechanisms

```rust
// crates/mod-api/src/override_api.rs (new, unconditional — no feature gate; both
// tiers and both mechanisms (behavior, system) share this one small enum)
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum OverrideMode { Wrap, Replace }

/// `before`/`after` ordering against another mod's own override of the identical
/// target (MOD-D38) — a flat `ModId` list, never a full `HookOrderRef` (only one
/// target is ever in play per call, so a `native:<domain>`-shaped marker has no
/// meaning here; "vanilla's own original" is always, implicitly, innermost).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OverrideOrder { pub before: Vec<ModId>, pub after: Vec<ModId> }
```

### `ModOriginalBlockBehavior<'a>` — the tier-transparent call-original handle

MOD-D35: "the handle is an ABI-stable closure... capturing whatever the previous layer's dispatch actually was." Mirrors `ModUpdateContext`'s own per-method closure-bundling pattern (M8-B01) exactly, one closure per `ModBlockBehavior` method, each additionally taking `&mut ModUpdateContext` as its first parameter (the *same* context instance flowing through the whole call for this one invocation — never captured, always supplied fresh, matching MOD-D17's "nothing authoritative survives across calls" rule). **Moderate-confidence flag, re-verify at implementation time** (mirroring M8-B01's own identical, already-sanctioned flag for `stabby::closure`'s exact generic arity/naming): this blueprint assumes `CallMutN<'a, Arg1..ArgN, Ret>` accepts `&mut ModUpdateContext` as an ordinary `ArgK` — no Deliverable signature's *shape* changes if a detail differs, only exact generic-parameter spelling.

```rust
// crates/mod-api/src/override_api.rs (continued), native-tier feature
use crate::block_behavior::ModUpdateContext;
use crate::geometry::{ModBlockPos, ModDirection};
use crate::registry::ModBlockStateId;

#[stabby::stabby]
pub struct ModOriginalBlockBehavior<'a> {
    on_neighbor_changed: stabby::closure::CallMut3<'a, &'a mut ModUpdateContext<'a>, ModBlockPos, ModDirection, ()>,
    on_shape_update: stabby::closure::CallMut4<'a, &'a mut ModUpdateContext<'a>, ModBlockPos, ModDirection, ModBlockStateId, stabby::option::Option<ModBlockStateId>>,
    on_scheduled_tick: stabby::closure::CallMut2<'a, &'a mut ModUpdateContext<'a>, ModBlockPos, ()>,
    on_block_event: stabby::closure::CallMut5<'a, &'a mut ModUpdateContext<'a>, ModBlockPos, u8, u8, ModBlockStateId, ()>,
}

impl<'a> ModOriginalBlockBehavior<'a> {
    pub fn on_neighbor_changed(&mut self, ctx: &mut ModUpdateContext, pos: ModBlockPos, from: ModDirection);
    pub fn on_shape_update(&mut self, ctx: &mut ModUpdateContext, pos: ModBlockPos, from: ModDirection, neighbor_state: ModBlockStateId) -> Option<ModBlockStateId>;
    pub fn on_scheduled_tick(&mut self, ctx: &mut ModUpdateContext, pos: ModBlockPos);
    pub fn on_block_event(&mut self, ctx: &mut ModUpdateContext, pos: ModBlockPos, event_id: u8, event_param: u8, block_state: ModBlockStateId);
}

/// A `Wrap`-mode override's own trait — distinct from `ModBlockBehavior` (M8-B01,
/// used for both ordinary new-block registration and `Replace`-mode overrides, which
/// need no original handle at all). Every method mirrors `ModBlockBehavior`'s own
/// four one-for-one, with one added parameter: the live handle to whatever this
/// override targeted.
#[stabby::stabby]
pub trait ModBlockBehaviorWrap: Send + Sync {
    fn on_neighbor_changed(&self, ctx: &mut ModUpdateContext, pos: ModBlockPos, from: ModDirection, original: &mut ModOriginalBlockBehavior) {}
    fn on_shape_update(&self, ctx: &mut ModUpdateContext, pos: ModBlockPos, from: ModDirection, neighbor_state: ModBlockStateId, original: &mut ModOriginalBlockBehavior) -> stabby::option::Option<ModBlockStateId> { stabby::option::Option::None }
    fn on_scheduled_tick(&self, ctx: &mut ModUpdateContext, pos: ModBlockPos, original: &mut ModOriginalBlockBehavior) {}
    fn on_block_event(&self, ctx: &mut ModUpdateContext, pos: ModBlockPos, event_id: u8, event_param: u8, block_state: ModBlockStateId, original: &mut ModOriginalBlockBehavior) {}
}
```

### `RegistryBuildContext`'s two new recording methods

Mirrors M8-B02's own "recording structure, never a live callback" resolution exactly (Context, above): both calls append to a new `Vec` field, translated into real engine state by a later pass — never during the mod's own `on_registry_build` call.

```rust
// crates/mod-api/src/entrypoint.rs (modify — additive only)
pub struct RegistryBuildContext {
    // ...existing fields (M8-B01/B02) unchanged...
    behavior_overrides: Vec<RecordedBehaviorOverride>,
}
impl RegistryBuildContext {
    /// MOD-D35 `Replace`. `target`'s namespace need not equal this mod's own
    /// `[mod].id` — an override, by definition, targets someone else's (or
    /// vanilla's) content.
    pub fn override_block_behavior_replace(&mut self, target: Identifier, behavior: stabby::dynptr!(stabby::boxed::Box<dyn ModBlockBehavior>), order: OverrideOrder);
    /// MOD-D35 `Wrap`.
    pub fn override_block_behavior_wrap(&mut self, target: Identifier, behavior: stabby::dynptr!(stabby::boxed::Box<dyn ModBlockBehaviorWrap>), order: OverrideOrder);
}

/// One recorded override request (Context: recording, never live). `mode` is
/// implicit in which variant a caller matches on — kept as an explicit field
/// alongside the boxed behavior (rather than folding it into the enum's own shape)
/// so `RecordedBehaviorOverride` stays a single, uniform type `RecordedRegistrations`
/// can hold in one `Vec` regardless of mode.
pub enum RecordedBehaviorOverride {
    Replace { target: Identifier, behavior: stabby::dynptr!(stabby::boxed::Box<dyn ModBlockBehavior>), order: OverrideOrder },
    Wrap { target: Identifier, behavior: stabby::dynptr!(stabby::boxed::Box<dyn ModBlockBehaviorWrap>), order: OverrideOrder },
}
```

`RecordedRegistrations` (M8-B02) gains one field, `pub behavior_overrides: Vec<RecordedBehaviorOverride>`, appended after its existing `channels` field; `into_recorded()`'s own body gains one additional move, no other line changes.

### The adapter — `rc-mechanics`'s new `mod_behavior_adapter.rs`

Three pieces, in dependency order:

**1. `build_mod_update_context` — completing M8-B01's own deferred construction.** Bundles five closures over a real `&mut UpdateContext<'_>`, converting `ModBlockPos`↔`BlockPos` (already-established `From`/`Into` pair, M8-B01's `geometry.rs`), `ModBlockStateId`↔`BlockStateId` (newtype `.0`/tuple-struct construction, both plain `u32` wrappers), `TickPriority`↔`TickPriority` (already-established one-for-one mirror, M8-B01's own Context), and `BlockEvent { pos, event_id, event_param, block_state }` (M3-B01) constructed inline from `emit_block_event`'s four plain arguments.

```rust
// crates/mechanics/src/mod_behavior_adapter.rs (new)
use rc_mechanics::behavior::UpdateContext;

/// The real, engine-wired counterpart to M8-B04's own test-only, hand-rolled
/// `ModUpdateContext::new` usage (Context: "the missing adapter this blueprint
/// builds"). Valid for exactly the duration of `f`'s own call — never stored.
pub fn with_mod_update_context<'a, R>(
    ctx: &'a mut UpdateContext<'a>,
    f: impl FnOnce(&mut rc_mod_api::ModUpdateContext<'a>) -> R,
) -> R;
```

**2. `ModBlockBehaviorAdapter` — real `BlockBehavior` wrapping a `Replace`-mode (or ordinary new-block) mod behavior.**

```rust
use rc_mechanics::behavior::{BlockBehavior, UpdateContext};
use rc_core::BlockPos;

pub struct ModBlockBehaviorAdapter {
    inner: stabby::dynptr!(stabby::boxed::Box<dyn rc_mod_api::ModBlockBehavior>),
}
impl ModBlockBehaviorAdapter {
    pub fn new(inner: stabby::dynptr!(stabby::boxed::Box<dyn rc_mod_api::ModBlockBehavior>)) -> Self;
}
impl BlockBehavior for ModBlockBehaviorAdapter {
    // Each method: with_mod_update_context(ctx, |mod_ctx| self.inner.on_*(mod_ctx, pos.into(), ...))
    // with position/state/direction conversions per Context above.
}
```

**3. `ModBlockBehaviorWrapAdapter` — real `BlockBehavior` wrapping a `Wrap`-mode mod behavior plus its captured original.**

```rust
pub struct ModBlockBehaviorWrapAdapter {
    inner: stabby::dynptr!(stabby::boxed::Box<dyn rc_mod_api::ModBlockBehaviorWrap>),
    /// Whatever this `Wrap` targeted — vanilla's own real behavior, or (for a
    /// non-innermost layer in a multi-mod chain) the adapter one layer further in.
    original: std::sync::Arc<dyn BlockBehavior>,
}
impl ModBlockBehaviorWrapAdapter {
    pub fn new(inner: stabby::dynptr!(stabby::boxed::Box<dyn rc_mod_api::ModBlockBehaviorWrap>), original: std::sync::Arc<dyn BlockBehavior>) -> Self;
}
impl BlockBehavior for ModBlockBehaviorWrapAdapter {
    // Each method: constructs a ModOriginalBlockBehavior whose 4 closures call
    // self.original's own matching BlockBehavior method (through the identical
    // with_mod_update_context bridging), then calls self.inner.on_*(mod_ctx, pos, ..., &mut original).
}
```

**Composing a resolved chain into one installable `Arc<dyn BlockBehavior>` — innermost first, mirroring `ResolvedOverrideChain.chain`'s own outermost-first order read backwards.** `apply_override_chain` (Deliverables, `behavior.rs`) walks `chain` from its *last* element (innermost) to its *first* (outermost): the innermost entry is wrapped around whatever `BlockBehaviorRegistry::resolve_named(target)` already returned *before* this call began (vanilla's own original, or an earlier already-installed override — MOD-D35's own "vanilla native logic, vanilla data-driven logic, or an earlier mod's own prior registration"); a `Replace` entry (there is at most one, always `chain`'s own last element per the algorithm above) discards that prior original entirely, becoming the new innermost `Arc` with no wrapping; each subsequent (more outer) `Wrap` entry becomes a `ModBlockBehaviorWrapAdapter` whose `original` is the `Arc` built by the previous (more inner) step. The final, fully-composed `Arc<dyn BlockBehavior>` (`chain`'s first/outermost element's own adapter) is what `override_named_range` installs.

### `BlockBehaviorRegistry`'s new methods

```rust
// crates/mechanics/src/behavior.rs (modify — additive only; register_range/
// register_one/resolve's own bodies and signatures are byte-identical, unchanged)
impl BlockBehaviorRegistry {
    /// First-time, named registration (Context: "the engine-side name -> range
    /// table"). Calls `register_range` internally — every existing overlap-panic
    /// property is preserved exactly; additionally records `id -> (start, end)`.
    pub fn register_named_range(&mut self, id: Identifier, start: BlockStateId, end_exclusive: BlockStateId, behavior: Arc<dyn BlockBehavior>);
    pub fn resolve_named(&self, id: &Identifier) -> Option<(BlockStateId, BlockStateId)>;
    /// Installs `composed` (Context: "Composing a resolved chain") over every state
    /// in `id`'s own already-`register_named_range`d span. Never panics on overlap —
    /// occupying an already-occupied named target is this method's entire purpose
    /// (MOD-D35's own "the only legal way to occupy an already-occupied target").
    pub fn override_named_range(&mut self, id: &Identifier, composed: Arc<dyn BlockBehavior>) -> Result<(), BehaviorOverrideError>;
    /// MOD-D34's discoverability requirement, server-side data layer (Context,
    /// "Where this blueprint's boundary falls"). Empty for an unmodded registry.
    pub fn active_overrides(&self) -> Vec<BehaviorOverrideRecord>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BehaviorOverrideRecord { pub target: Identifier, pub layers: Vec<(rc_mod_api::ModId, rc_mod_api::OverrideMode)> }

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BehaviorOverrideError {
    #[error("override target {0} was never registered via register_named_range — MOD-D35's own targeting requires a real, named registration to already exist (nothing to override)")]
    UnknownTarget(Identifier),
}
```

`register_named_range`/`resolve_named`/`override_named_range`/`active_overrides` all live on the *same* `crates/mechanics/src/behavior.rs` file M3-B01 already shipped — additive methods on the already-`pub struct BlockBehaviorRegistry`, plus one new private field (`names: HashMap<Identifier, (BlockStateId, BlockStateId)>`) and one new private field tracking installed override layers (for `active_overrides`'s own bookkeeping). `Identifier` is `rc_mod_api::Identifier`, reached through the new `rc-mod-api` dependency (Context, above).

### MOD-D36/D37 — named systems, disable, and in-place replace

MOD-D36, restated exactly: "native systems gain stable public identifiers, exported explicitly by whichever document owns them" — the identical shape to `export_component` (M8-B03), generalized from components to systems. MOD-D37: a mod may **disable** a named system entirely (never scheduled, no `Access` declaration, no conflict-graph entry) or **replace** it with a mod-supplied system declaring its own `Access<ComponentId>`, "slotted into the identical group/stage the disabled original occupied" — this blueprint's own binding reading of "identical... slot" is literal position preservation, not merely group membership: a replacement takes over the *exact* `order_tag` the original held, rather than being appended at the end of its group's own registration sequence, avoiding any accidental reordering relative to every sibling system that was never touched.

```rust
// crates/scheduler/src/registry.rs (modify — additive only)
use rc_mod_api::Identifier;

/// Mirrors `EngineComponentExports`'s own shape exactly (M8-B03), generalized from
/// components to systems (MOD-D36).
pub struct NamedSystemExports {
    by_name: std::collections::HashMap<Identifier, SystemId>,
}
impl NamedSystemExports {
    pub fn resolve(&self, name: &Identifier) -> Option<SystemId>;
    pub fn names(&self) -> impl Iterator<Item = &Identifier>;
}

impl RcExecutorBuilder {
    /// Registers `factory` into `group` exactly as `register_system` already does,
    /// additionally recording it under the stable name `id` (Context: MOD-D36).
    /// Naming a system twice is a caller bug (`ExecutorBuildError::
    /// DuplicateSystemExport`).
    pub fn register_named_system(&mut self, id: Identifier, group: DomainGroup, factory: SystemFactory, structural_writes: Vec<ComponentId>) -> SystemId;

    /// MOD-D37 disable: `id` must already be `register_named_system`-exported
    /// (`Err(UnknownSystemTarget)` otherwise). The targeted `Registration` is marked
    /// suppressed — `build()`'s own conflict-graph pass (Context, below) excludes it
    /// entirely: no `Access` declaration, no `compute_waves` participation, no
    /// dispatch. A redundant disable on an already-disabled/replaced target is a
    /// no-op (MOD-D37's own "harmless no-op" rule, extended verbatim from the
    /// hook case).
    pub fn disable_named_system(&mut self, id: &Identifier) -> Result<(), ModSystemTargetError>;

    /// MOD-D37 replace: overwrites the named system's own `Registration` in place —
    /// same `group`, same `order_tag` (Context: "literal position preservation") —
    /// with `factory`/`declared_access`/`structural_writes` a future orchestrator
    /// resolves exactly as it resolves a generic hook's own access (M8-B03's
    /// `resolve_component_access`, reused unmodified, not re-implemented here).
    pub fn replace_named_system(
        &mut self,
        id: &Identifier,
        factory: SystemFactory,
        access: crate::access::ComponentAccessSummary,
        structural_writes: Vec<ComponentId>,
    ) -> Result<(), ModSystemTargetError>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ModSystemTargetError {
    #[error("system {0} was never exported via register_named_system — MOD-D36's own opt-in requires a real export to already exist")]
    UnknownSystemTarget(Identifier),
}
```

`ExecutorBuildError` gains two variants: `DuplicateSystemExport(Identifier)` (mirrors `DuplicateComponentExport` exactly) and reuses `AmbiguousMutationAuthority` unmodified for a replacement's own access-vs-structural-writes conflict, identically to how M8-B03 already reused it for mod hooks.

**`build()`'s algorithm gains one additive step**, positioned identically to M8-B03's own two additive steps (Context there): immediately after resolving `EngineComponentExports`/`NamedSystemExports` (both computed the same way, against the same prototype `World`, in the same pass) and *before* `compute_waves` runs per group, every `disable_named_system`-targeted `Registration` is filtered out of its group's own registration list entirely (never reaches `compute_waves`, never occupies a wave, never gets a `SystemId` in the final `CompiledGroup`); every `replace_named_system`-targeted `Registration`'s stored `factory`/`access`/`structural_writes` are overwritten in place at that same array index, before `compute_waves` is computed for that group — so the replacement genuinely participates in that group's own conflict graph with its *own* declared access, at its original's own `order_tag` position, exactly once. `RcExecutor` gains `named_system_exports: NamedSystemExports` (mirrors `component_exports`) and `pub fn named_system_exports(&self) -> &NamedSystemExports` plus `pub fn active_system_overrides(&self) -> Vec<SystemOverrideRecord>` (MOD-D34, Context above):

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemOverrideRecord { pub target: Identifier, pub kind: SystemOverrideKind }
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SystemOverrideKind { Disabled, Replaced }
```

**Why Stage 4's own single-worker collapse needs no special-casing here.** A replacement slotted into the `BlockRedstone` group is dispatched by the *identical* Stage-4-specific dispatch path M0-B05/M3-B01 already established (ARCH-D13's mandatory full-drain collapse, unconditioned on system origin) — `tick_region`'s own dispatch logic is untouched by this blueprint (mirroring M8-B03's own "Why `executor.rs`'s dispatch logic needs zero changes"), so a replacement system occupying that group is, from `tick_region`'s point of view, indistinguishable from any native Stage-4 system and inherits sequential collapse automatically, with zero new code.

### MOD-D34 — parity-opt-out surfacing, scoped honestly

MOD-D34: "whenever any loaded mod has engaged a behavior-level or system-level override/replace/disable... the server's `minecraft:brand` response... carries a documented suffix... and the Server List Ping status response gains a small... indicator — both purely informational, never a connection gate." This blueprint implements the **data layer only** — `BlockBehaviorRegistry::active_overrides()` and `RcExecutor::active_system_overrides()`, both real, queryable, correct — and explicitly does **not** touch any packet, since 06's own Interfaces section already names the wire-format gap as unresolved ("the `minecraft:brand` channel's own exact field shape/timing... `02`'s current decision table does not yet name it explicitly"). A future `02-protocol-networking.md`-owned blueprint is this mechanism's honest, named consumer, exactly mirroring how M8-B01/B02/B04 each honestly deferred WASM-tier hosting to "a future, not-yet-numbered `rc-mod-host` blueprint" rather than faking it. **Restated exactly what these two queries exclude, per MOD-D34's own text:** the event layer's own observation-only reach (MOD-D39, `M8-B06b`) never counts, "since it never replaces vanilla logic"; a purely additive mod (new blocks/items/hooks, no override of any kind) reports empty from both queries, exactly as an unmodded server does.

### Determinism, cluster compatibility — restated, unchanged

Both mechanisms inherit every determinism/cluster-transparency guarantee M8-B03 already established for `ModSystemShim`, with zero special-casing: a replacement system's wave membership is fixed by the identical `compute_waves` algorithm over its own declared `ComponentAccessSummary`; a behavior-override adapter's own dispatch happens entirely inside Stage 4's already-sequential, already-synchronous call (never a separate scheduling concern of its own — block-behavior dispatch was never conflict-graph-scheduled in the first place, M8-B03's own Context: "no new conflict-graph entry... Stage 4's existing single-worker sequential collapse already covers it for free"). Neither mechanism introduces a `NodeId`/host-identity value anywhere in its own public surface (MOD-D15, unchanged), and neither ever blocks a tick waiting on another partition (MOD-D16, unchanged) — both are ordinary, synchronous, in-process Rust calls.

## Deliverables

### `crates/mod-api/src/override_api.rs` (new, mixed feature gating: `OverrideMode`/`OverrideOrder` unconditional, everything else `native-tier`)

Full contents per Context above: `OverrideMode`, `OverrideOrder`, `ModOriginalBlockBehavior<'a>` (+ inherent methods), `ModBlockBehaviorWrap` trait.

### `crates/mod-api/src/entrypoint.rs` (modify)

`RegistryBuildContext` gains the `behavior_overrides` field and the two `override_block_behavior_*` methods; new `RecordedBehaviorOverride` enum; `RecordedRegistrations` gains the `behavior_overrides` field.

### `crates/mod-api/src/lib.rs` (modify)

```rust
mod override_api;
pub use override_api::{OverrideMode, OverrideOrder};
#[cfg(feature = "native-tier")]
pub use override_api::{ModBlockBehaviorWrap, ModOriginalBlockBehavior};
// existing entrypoint:: pub use line gains RecordedBehaviorOverride, appended
```

### `crates/scheduler/src/mod_order.rs` (modify)

Adds `OverrideOrderInput`, `ResolvedOverrideChain`, `OverrideOrderingError`, `resolve_override_order` per Context above, alongside M8-B03's existing `HookOrderInput`/`ResolvedHookOrder`/`ModOrderingError`/`resolve_hook_order`, all unmodified.

### `crates/scheduler/src/registry.rs` (modify)

`NamedSystemExports`, `RcExecutorBuilder::register_named_system`/`disable_named_system`/`replace_named_system`, `ModSystemTargetError`, two new `ExecutorBuildError` variants, per Context above. `build()`'s algorithm gains the one additive filter/overwrite step described in Context.

### `crates/scheduler/src/executor.rs` (modify)

`RcExecutor` gains `named_system_exports: NamedSystemExports` field, `pub fn named_system_exports(&self) -> &NamedSystemExports`, `pub fn active_system_overrides(&self) -> Vec<SystemOverrideRecord>`; new `SystemOverrideRecord`/`SystemOverrideKind` types.

### `crates/scheduler/src/lib.rs` (modify)

```rust
pub use mod_order::{
    // existing names unchanged, plus:
    OverrideOrderInput, OverrideOrderingError, ResolvedOverrideChain, resolve_override_order,
};
pub use registry::{
    // existing names unchanged, plus:
    ModSystemTargetError, NamedSystemExports,
};
pub use executor::{SystemOverrideKind, SystemOverrideRecord};
```

### `crates/mechanics/Cargo.toml` (modify — add two lines)

```toml
[dependencies]
rc-mod-api = { path = "../mod-api", default-features = false, features = ["native-tier"] }
stabby = { workspace = true }
```

### `crates/mechanics/src/behavior.rs` (modify)

`BlockBehaviorRegistry` gains `names`/override-bookkeeping private fields, `register_named_range`, `resolve_named`, `override_named_range`, `active_overrides`; new `BehaviorOverrideRecord`/`BehaviorOverrideError` types, per Context above. `register_range`/`register_one`/`resolve`'s existing bodies are byte-identical, unmodified.

### `crates/mechanics/src/fluid.rs` (modify — two call sites only)

`register_fluids`'s body: the two `registry.register_range(...)` calls for water and lava become `registry.register_named_range(id("minecraft:water"), ...)` / `id("minecraft:lava")`. No other line changes; `FluidBehavior`'s own `impl BlockBehavior` is untouched.

### `crates/mechanics/src/mod_behavior_adapter.rs` (new)

`with_mod_update_context`, `ModBlockBehaviorAdapter`, `ModBlockBehaviorWrapAdapter`, and the chain-composition logic (`apply_override_chain`, called by `override_named_range`'s own caller — Deliverables note: `apply_override_chain` itself lives here, taking a `&mut BlockBehaviorRegistry` and a `ResolvedOverrideChain` plus the originating `entries` slice, and is the one function that actually calls `override_named_range`):

```rust
pub fn apply_override_chain(
    registry: &mut BlockBehaviorRegistry,
    target: &Identifier,
    entries: &[(rc_mod_api::ModId, RecordedBehaviorOverride)],
    resolved: &rc_scheduler::ResolvedOverrideChain,
) -> Result<(), BehaviorOverrideError>;
```

### `crates/mechanics/src/lib.rs` (modify)

```rust
mod mod_behavior_adapter;
pub use mod_behavior_adapter::{apply_override_chain, ModBlockBehaviorAdapter, ModBlockBehaviorWrapAdapter, with_mod_update_context};
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46), restated exactly, matching every prior M8 blueprint's identical rule.** The test-authoring changeset is every file below, plus every new/modified `src/*.rs` file from Deliverables with each function body replaced by `todo!()` (field lists, derives, doc comments stay exactly as specified), plus the three `Cargo.toml` edits. The implementation changeset fills in bodies only — it must not modify any file under any of the three crates' `tests/` directories, must not change any type's field list/derive list/public signature, must not weaken any assertion below, and must not touch any pre-existing M8-B01/B02/B03/M3-B01/M4-B06 test file at all.

A shared `crates/scheduler/tests/common/override_fixtures.rs` (new) holds:

```rust
pub fn id(s: &str) -> Identifier { Identifier::parse(s).unwrap() }
pub fn mod_id(s: &str) -> ModId { ModId::new(s).unwrap() }
```

### `crates/scheduler/tests/override_order_resolution.rs` (pure)

1. `single_replace_produces_a_one_element_chain` — one `Replace` entry, no ordering; `chain == [0]`, `truncated`/`rejected` empty.
2. `wraps_compose_outermost_first_declaration_order` — three `Wrap` entries, no ordering, distinct `load_order_index` ascending 0,1,2; `chain == [0,1,2]`.
3. `explicit_before_reorders_two_wraps` — two `Wrap`s, entry 1 declares `before: [entries[0].mod_id]`; `chain == [1, 0]`.
4. `replace_truncates_every_inner_wrap` — chain resolves to `[Wrap, Replace, Wrap]` order (via explicit ordering); assert `chain == [0, 1]` (the trailing wrap dropped from `chain`) and `truncated == [2]`.
5. `two_unordered_replaces_are_rejected` — two `Replace` entries, no before/after between them; `Err(OverrideOrderingError::UnresolvedReplaceConflict { mods, .. })` naming both — the double-override diagnostic this blueprint's own Goal & Done names explicitly.
6. `two_explicitly_ordered_replaces_resolve_deterministically` — as above but entry 1 declares `after: [entries[0].mod_id]`; `Ok`, `chain == [1]`, `rejected == [0]`.
7. `cycle_between_two_mods_is_rejected` — entry 0 `after: [mod_1]`, entry 1 `after: [mod_0]`; `Err(Cycle { mods, .. })` containing both, order-independent (`HashSet` comparison).
8. `unknown_reference_is_rejected` — entry 0's `before` names a `ModId` absent from `entries`; `Err(UnknownOrderingTarget { .. })`.
9. `empty_entries_returns_empty_chain` — `resolve_override_order(target, &[])` is `Ok(ResolvedOverrideChain { chain: vec![], truncated: vec![], rejected: vec![] })`.

### `crates/scheduler/tests/named_system_override.rs` (integration — real `RcExecutorBuilder`)

Uses synthetic systems (no `bevy_ecs::World` component reads beyond a marker type), mirroring M0-B05/M8-B03's own precedent.

1. `disabled_named_system_is_excluded_from_compute_waves` — register two systems into a chunk-parallel group (`Lighting`), one named and one anonymous, disjoint access; `disable_named_system` the named one; `build()`'s resulting group has exactly one `CompiledSystem` (the anonymous one) — the disabled one contributes no wave membership at all.
2. `replaced_named_system_keeps_its_original_order_tag_and_group` — register three named systems into `Lighting` in order A, B, C (`order_tag` 0,1,2 respectively, all mutually conflicting on one shared component so `compute_waves` serializes them `[[0],[1],[2]]`); `replace_named_system("B", ...)` with a new factory/access still conflicting with both A and C; assert the built executor's wave structure is unchanged in shape (`[[0],[1],[2]]`) and that invoking wave index 1 now runs the *replacement*'s own factory-produced system, not B's original (a shared `AtomicU32` call-counter test double distinguishes them).
3. `stage_4_replacement_still_dispatches_single_worker_sequential` — register a named system into `BlockRedstone` (Stage 4), replace it; `tick_region` (M0-B05's own synchronous test driver) dispatches it via the identical Stage-4-specific single-worker path every other Stage-4 system uses (asserted the same way M0-B05's own `sync_points.rs`/ARCH-D13 tests already assert this — a shared, non-atomic `Cell<u32>` counter mutated with no synchronization primitive by the replacement's own factory-produced system does not corrupt under Miri/thread-sanitizer-style reasoning specifically because only one worker ever touches Stage 4, matching ARCH-D13's own already-proven guarantee, unmodified by this blueprint).
4. `disabling_an_unexported_target_is_rejected` — `disable_named_system(&id("nonexistent:system"))` on a builder that never exported that name: `Err(ModSystemTargetError::UnknownSystemTarget(_))`.
5. `redundant_disable_on_already_replaced_target_is_a_no_op` — replace a named system, then disable the same `id`; `build()` still succeeds, the replacement (not a disabled no-op) is what actually runs — MOD-D37's own "harmless no-op... a replacement already implies the original no longer runs."
6. `duplicate_named_export_is_rejected` — `register_named_system` twice with the same `id`; `build()` returns `Err(ExecutorBuildError::DuplicateSystemExport(_))`.

### `crates/scheduler/tests/override_activity_report.rs` (integration)

1. `unmodded_executor_reports_no_system_overrides` — a builder with only ordinary `register_system`/`register_named_system` calls, no disable/replace; `built.active_system_overrides()` is empty.
2. `disabled_and_replaced_systems_both_appear_with_correct_kind` — one `disable_named_system`, one `replace_named_system` on two distinct exported names; `active_system_overrides()` contains exactly two records, `SystemOverrideKind::Disabled` and `::Replaced` respectively, each naming the correct target `Identifier`.

### `crates/mechanics/tests/mod_behavior_adapter.rs` (pure/unit — no `RcExecutor`, no dylib)

Uses a hand-rolled `TestWorld: BlockWorldAccess` (a plain `HashMap<BlockPos, BlockStateId>`, mirroring M3-B01's own test convention) to construct a real `UpdateContext<'a>`.

1. `replace_adapter_dispatches_into_the_boxed_mod_behavior` — a `ModBlockBehaviorAdapter` wrapping a hand-written `ModBlockBehavior` impl that records every call into a `Rc<RefCell<Vec<String>>>`; call `on_scheduled_tick` through the adapter's own `BlockBehavior::on_scheduled_tick`; assert the mod's own method ran exactly once with the correct `pos`.
2. `wrap_adapter_original_handle_genuinely_invokes_the_previous_behavior` — a `ModBlockBehaviorWrapAdapter` wrapping a mod behavior whose `on_scheduled_tick` calls `original.on_scheduled_tick(ctx, pos)` unconditionally, with `original` set to a plain `Arc<dyn BlockBehavior>` test double (a `LoggingBehavior` recording its own calls, mirroring M3-B01's own `LoggingBehavior` test-double convention); assert the *original*'s own log recorded exactly one call — "original callable," this blueprint's own Goal & Done wording, proven directly.
3. `wrap_adapter_may_choose_not_to_call_original` — as above, but the mod behavior's own `on_scheduled_tick` never touches `original`; assert the original's log is empty — MOD-D35's own "before, after, conditionally, or not at all."
4. `position_and_state_conversions_round_trip` — `ModBlockPos`↔`BlockPos` and `ModBlockStateId`↔`BlockStateId` round-trip bit-exactly for several sample values (property-style, `proptest`, already a `rc-mechanics` dev-dependency per M3-B01).

### `crates/mechanics/tests/water_override_replace.rs` (integration — the flagship demo)

Builds a real `FluidTables`/`WaterloggableRegistry`/`LevelRandom` (M4-B06's own already-shipped test-construction helpers) and a real `BlockBehaviorRegistry` with `register_fluids` called for real water/lava ranges.

1. `water_resolves_to_the_real_fluid_behavior_before_any_override` — `registry.resolve_named(&id("minecraft:water"))` is `Some((start, end))` matching `tables.ranges.water`; `registry.resolve(start)` is the `FluidBehavior` instance `register_fluids` installed (identity-checked via `Arc::ptr_eq` against a value captured immediately after `register_fluids` returns).
2. `replace_override_installs_and_is_what_resolve_now_returns` — construct a test `ReplacementFluidBehavior` (`impl rc_mod_api::ModBlockBehavior` — `on_scheduled_tick` increments a shared counter and calls `ctx.set_block` to a *fixed*, non-spreading marker state instead of anything resembling `spread`); build one `OverrideOrderInput { mode: Replace, .. }`, resolve via `resolve_override_order`, apply via `apply_override_chain`; `registry.resolve(start)` is now a `ModBlockBehaviorAdapter` (identity-checked as *not* `Arc::ptr_eq` to the pre-override `FluidBehavior`) for **every** `BlockStateId` in `[start, end)`.
3. `custom_spread_is_observable_and_vanilla_spread_never_runs` — construct a real `UpdateContext` over a small `TestWorld` with a water source at the origin and empty air neighbors; invoke `registry.resolve(source_state).on_scheduled_tick(&mut ctx, origin)` (the exact call Stage 4's own real dispatch loop makes, restated from M3-B01's `run_scheduled_phase`); assert every neighbor position is **still empty** (vanilla `spread` — which unconditionally fans out to neighbors even for source cells, M4-B06's own Context §D — never ran) and the origin's own new state is the replacement's fixed marker state, and the shared counter incremented exactly once — "custom spread observable... the engine provably uses it," proven against the real `FluidBehavior`/`BlockBehaviorRegistry` machinery, not a synthetic stand-in.
4. `wrap_override_of_water_still_lets_original_spread_run_when_invoked` — a second, independent scenario: a `Wrap`-mode override whose own logic calls `original.on_scheduled_tick(ctx, pos)` unconditionally before its own extra bookkeeping; assert neighbors *do* receive spread states (matching vanilla `FluidBehavior`'s own real algorithm, exercised transitively through the wrap) — proving `Wrap`'s call-original path is genuinely wired to the *real* fluid behavior, not merely the pure adapter-level double from `mod_behavior_adapter.rs`'s own tests.

### `crates/mechanics/tests/unmodded_parity_regression_guard.rs` (integration — the scoping proof)

1. `zero_overrides_leaves_water_behavior_byte_identical` — construct `registry`/`register_fluids` exactly as `water_override_replace.rs`'s own first test, capture `Arc::as_ptr(registry.resolve(water_start))`; perform *no* override call at all; re-read `registry.resolve(water_start)` — pointer-identical (`Arc::ptr_eq`), proving this blueprint's own machinery changes nothing about the default dispatch path when unused.
2. `unmodded_registry_reports_no_active_overrides` — `registry.active_overrides()` is empty on the same `registry`.
3. `unmodded_executor_reports_no_active_overrides` — reuses `named_system_override.rs`'s own `unmodded_executor_reports_no_system_overrides` case by direct call (not re-implemented — imported from the shared fixture module), matching this blueprint's own Constraints (a) discipline against duplicated test logic.

## Implementation steps

1. **`crates/mod-api/src/override_api.rs`.** Resolve the `stabby::closure` moderate-confidence flag (Context) against the installed `stabby` 72.1.16 crate first, then implement `OverrideMode`, `OverrideOrder`, `ModOriginalBlockBehavior<'a>`, `ModBlockBehaviorWrap`. Observable: `cargo build -p rc-mod-api --features native-tier` succeeds.
2. **`crates/mod-api/src/entrypoint.rs`.** Add `behavior_overrides` field, `override_block_behavior_replace`/`_wrap`, `RecordedBehaviorOverride`, extend `RecordedRegistrations`. Observable: M8-B01/B02's own full test suite still passes unmodified; new methods compile.
3. **`crates/mod-api/src/lib.rs`.** Add the new module/`pub use` lines. Observable: `cargo build -p rc-mod-api --all-features` succeeds with zero warnings.
4. **`crates/scheduler/src/mod_order.rs`.** Implement `resolve_override_order` per Context's four-step algorithm. Observable: `override_order_resolution.rs` passes in full — no other crate touched yet, so this step is independently verifiable.
5. **`crates/scheduler/src/registry.rs` + `executor.rs`.** Implement `NamedSystemExports`, `register_named_system`/`disable_named_system`/`replace_named_system`, the additive `build()` step, `active_system_overrides`. Observable: `named_system_override.rs` and `override_activity_report.rs` pass in full.
6. **`crates/scheduler/src/lib.rs`.** Add the new `pub use` lines. Observable: `cargo build -p rc-scheduler --all-features` succeeds with zero warnings; M8-B03's own full suite still passes unmodified.
7. **`crates/mechanics/Cargo.toml`.** Add the two new dependency lines. Observable: `cargo metadata` resolves cleanly; no new transitive dependency outside the pinned `[workspace.dependencies]` set.
8. **`crates/mechanics/src/behavior.rs`.** Add `names`/override-bookkeeping fields, `register_named_range`, `resolve_named`, `override_named_range`, `active_overrides`, `BehaviorOverrideRecord`/`BehaviorOverrideError`. Observable: M3-B01's own full test suite still passes unmodified (no existing method's body changed).
9. **`crates/mechanics/src/mod_behavior_adapter.rs`.** Implement `with_mod_update_context`, `ModBlockBehaviorAdapter`, `ModBlockBehaviorWrapAdapter`, `apply_override_chain`. Observable: `mod_behavior_adapter.rs`'s own four tests pass.
10. **`crates/mechanics/src/fluid.rs`.** Change the two `register_range` call sites to `register_named_range`. Observable: M4-B06's own full test suite still passes unmodified (the change is observationally invisible to every existing assertion, which never called `resolve_named`).
11. **`crates/mechanics/src/lib.rs`.** Add the new module/`pub use` lines. Observable: `cargo build -p rc-mechanics` succeeds with zero warnings.
12. **Final integration.** `water_override_replace.rs` and `unmodded_parity_regression_guard.rs` pass in full — the flagship end-to-end proof and the scoping guard, both against real M3-B01/M4-B06 machinery.

## Constraints & forbidden actions

(a) The implementation changeset (steps 1–12 above) must not modify any file under `crates/mod-api/tests/`, `crates/scheduler/tests/`, or `crates/mechanics/tests/`, must not change any type's field list/derive list/public signature from what the Acceptance tests section and Deliverables section fix, and must not weaken any assertion in this blueprint's own test changeset. It must not touch any pre-existing M8-B01/B02/B03/M3-B01/M4-B06 test file, fixture, or verification-tooling path (TEST-D46's protected-path list) at all — every one of this blueprint's own edits to `fluid.rs`/`behavior.rs`/`registry.rs`/`executor.rs`/`entrypoint.rs` is additive, and every pre-existing test in every touched crate must still pass unmodified, proving that additivity mechanically.

(b) No new external dependency beyond `12-workspace-structure.md`'s pinned `[workspace.dependencies]` set: `rc-mod-api` (already pinned as a path dependency), `stabby` (already pinned, `72.1.16`, `Apache-2.0` branch selected per MOD-D3). `rc-mechanics` gains exactly these two new lines, nothing else.

(c) No Mojang or third-party reimplementation source is consulted or copied anywhere in this blueprint's own Context or Deliverables — every algorithm above (`resolve_override_order`'s Kahn's-algorithm topological sort, the wrap-chain composition order) is this project's own original design, extending MOD-D10's already-established mechanism per MOD-D38's own binding instruction, never derived from any other modding ecosystem's source (ASSET-D18/D19/D30).

(d) `ModBlockBehaviorAdapter`/`ModBlockBehaviorWrapAdapter`/`apply_override_chain` never call `std::panic::catch_unwind` anywhere — crash isolation for a real, dylib-loaded native mod's own panicking behavior remains entirely `rc-mod-host`'s job (MOD-D32, M8-B02, unmodified), exactly as M8-B03's own `ModSystemShim` never calls it either. This blueprint's own acceptance tests never load a real dylib, so this constraint is exercised only by inspection (Constraints review), not by a dedicated panic-catching test — a future composition-root/`rc-mod-host` blueprint that wires a *real* loaded mod's override into this mechanism is responsible for wrapping the mod's own boxed trait-object calls in `catch_unwind` at that point, mirroring `native_mod_hook_invoke`'s own already-established pattern (M8-B04) exactly.

(e) `BlockBehaviorRegistry::register_range`/`register_one`/`resolve`'s existing bodies, panic behavior, and public signatures are never modified — every new method is additive, verified mechanically by M3-B01's own full test suite passing unmodified (Done definition).

(f) `resolve_override_order` is pure — it must never construct or reference a `bevy_ecs::World`, `RcExecutorBuilder`, or `BlockBehaviorRegistry` directly, so the identical function serves both this blueprint's own two independent call sites (behavior overrides in `rc-mechanics`, system overrides in `rc-scheduler`) with zero duplication.

## Verification commands

```
cargo nextest run -p rc-mod-api -p rc-scheduler -p rc-mechanics --profile ci
cargo test --doc -p rc-mod-api -p rc-scheduler -p rc-mechanics
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
```

All four commands must run headless and succeed on both `ubuntu-24.04` and `windows-2025` (TEST-D43), producing machine-readable output (TEST-D40) — nextest's own JUnit XML for the first two, an exit code for the latter two. CI tier: Tier 1 (TEST-D37), from a clean checkout (TEST-D50).
