# M8-B06b — Event Layer & Component Attachment to Vanilla Entities

| Field | Content |
|---|---|
| ID | M8-B06b |
| Milestone | M8 — Mod API Alpha |
| Prerequisites | M8-B01 (`rc-mod-api`'s complete public surface — `Identifier`, `ModId`, `RegistryBuildContext`/`RecordedRegistrations`, `ModBlockPos`, geometry mirrors — restated below); M8-B06a (`rc-mechanics`'s newly-added `rc-mod-api`/`stabby` dependency, Context — this blueprint's own component-attachment resolution functions live in the same crate and reuse that already-added dependency rather than re-adding it; no other coupling to M8-B06a's own override mechanism exists, matching 06's own "events sit underneath, composing freely but never depending on, the override tiers" framing); M8-B02/M8-B03 (restated as context only — this blueprint's own acceptance tests never load a dylib or drive a real `RcExecutor` tick); M3-B01 (`rc-mechanics`'s `ChunkIndex`, `BlockPos`, restated); M3-B06 (`rc-mechanics`'s `BlockEntityHeader`/block-entity-indexed-by-position convention, restated with a moderate-confidence flag — Context); M0-B02 (`rc-core`'s `ChunkKey`/`DimensionId`, restated in full) |
| Implements | MOD-D39 (cancellable/mutable events: observation and per-occurrence veto, priority tiers, monitor-tier read-only); MOD-D41 (persistence: the `ModComponents` NBT side-channel encoding, byte-exact, dormant-data-preserved); MOD-D42 (world-query resolution: `ChunkKey` → chunk entity, `BlockPos` → block entity) |
| Crates touched | `rc-mod-api` (`crates/mod-api/`), `rc-mechanics` (`crates/mechanics/`) |
| Estimated scope | L — part **b** of the two-part split of the task-level blueprint `M8-B06` (Context, `M8-B06a`'s own identical framing). This part covers the two mechanisms 06 itself describes as sitting independently *underneath* the override tiers (MOD-D39: "layered underneath — never a substitute for — the override tiers") and independently *beside* them (MOD-D41/D42: world-data persistence and resolution, no relationship to override/replace at all) — genuinely separable from `M8-B06a`'s own subject matter, hence the split boundary drawn here rather than elsewhere. |

## Goal & Done definition

Give mods MOD-D39's cancellable event layer — a per-occurrence, priority-ordered, five-tier-mutating-plus-monitor observation/veto mechanism, mechanically enforcing the monitor tier's own "cannot itself cancel or mutate" contract rather than merely documenting it — with the M8-alpha catalog's one illustrative entry (`BlockBreakAttempt`, 06's own first-named example) proven through a full cancellation matrix. Give mods MOD-D41's `ModComponents` persistence format — a byte-exact, dormant-data-preserving encoding requiring no per-component (de)serialize code, reusing MOD-D13's POD guarantee directly — with a real round-trip proof including the "uninstalled/version-mismatched data survives losslessly" property. Give mods MOD-D42's missing resolution step — `ChunkKey` → chunk entity (always present) and `BlockPos` → block entity (present only if one exists) — the piece MOD-D6/D13's own existing generic component-attachment machinery needs but never supplied, closing the gap between "a mod can define and attach a component to any `Entity`" and "a mod can obtain the `Entity` for a vanilla chunk or block position in the first place."

This blueprint does **not** implement: real dispatch of a *loaded, dylib-crossing* native-tier mod's own event listener (the pure dispatch engine here is real, engine-grade code; wiring it to a real `ServerModHost`-loaded mod's boxed trait object is deferred to "a future `rc-mod-host` blueprint," identically to `M8-B06a`'s own Constraints (d) deferral for override dispatch); a general native-tier "attach an arbitrary component to an arbitrary entity" call on `TickHookContext` (that type remains, per M8-B02's own already-established framing, "a pure marker with nothing for a mod to call on... yet" — this blueprint supplies only the two *resolution* functions MOD-D42 names, not a new live-attach mechanism, which M8-B01's WIT `world-query` interface already covers for the WASM tier and a future blueprint must still cover for native); the real `03-world-chunks-persistence.md`-owned placement of the reserved `ModComponents` compound tag inside a real chunk-column/block-entity/entity NBT record, or its wiring into `WORLD-D20`'s fast `ChunkSnapshot` path (06's own Interfaces section names both as still-open ratifications from `03`; this blueprint fixes only the *entry encoding contract* those integrations will consume); region-scoped (MOD-D43) or world-scoped (MOD-D44) mod data — both explicitly out of this blueprint's own scope (Context, "Scope boundary, restated").

Done when:

- [ ] `cargo build -p rc-mod-api --all-features` and `cargo build -p rc-mechanics` both succeed with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-mod-api -p rc-mechanics`.
- [ ] Every pre-existing test in both crates (M8-B01/B06a's, M3-B01/M3-B06's) still passes, byte-for-byte unmodified.
- [ ] `event_cancellation_matrix.rs` proves: a higher-tier cancellation is visible to every lower tier; a lower tier may reverse an earlier cancellation; every mutating listener runs regardless of the event's current cancellation state; FIFO-within-tier ordering; and that a monitor-tier listener's own attempted mutation is never observed by the real event or by any other listener — the mechanical enforcement, not merely documented.
- [ ] `mod_components_persistence.rs` proves byte-exact round-trip for a multi-entry tag, preservation of an entry whose namespace names a currently-unloaded mod, preservation of a version-mismatched entry, and a clean `Err` (never a panic) on a truncated byte stream.
- [ ] `mod_world_query.rs` proves `resolve_chunk_entity` returns the correct `ModEntityId` for a chunk present in a real `ChunkIndex` and `None` for an absent key; `resolve_block_entity` returns `Some` only where a block entity is actually indexed.
- [ ] `cargo run -p xtask -- lint-deps`, `fmt-check`, and `lint` all exit 0.
- [ ] `cargo test --doc -p rc-mod-api -p rc-mechanics` exits 0.
- [ ] CI tier: Tier 1 green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### MOD-D39, restated exactly

A named event fires once per occurrence, inline, synchronously, on whichever worker/stage is already executing the emitting system's own declared access — never a `ModSystemShim`, never a new `ARCH-D8` conflict-graph node. Listener priority is **five ordered mutating tiers** (Highest, High, Normal, Low, Lowest — FIFO within a tier by declaration order), run in fixed order every occurrence, plus a **sixth, always-last, always-read-only Monitor tier** that observes the event's final, possibly-cancelled outcome and cannot itself cancel or mutate. Any non-monitor listener may cancel the occurrence's default effect; every remaining listener, including lower-priority ones, still runs and may reverse or further adjust that outcome. Relationship to the override tiers (`M8-B06a`): events are observation and veto over a system that still runs its own logic; a system a mod has `Replace`d is never obligated to keep firing any event at all — the two compose freely but neither depends on the other, which is why this mechanism ships as an independent blueprint part rather than one requiring `M8-B06a` as a hard prerequisite.

### Enforcing "cannot itself cancel or mutate" mechanically, not by convention

06's own text states the Monitor tier's read-only property as a contract, without fixing a mechanism. A mod-facing listener trait needs one uniform method signature regardless of which tier it is registered at (a mod author does not know in advance, from inside their own `impl` block, which tier the manifest/registration call will place them at) — so the enforcement cannot come from a *different* trait signature for Monitor listeners. This blueprint's own binding resolution: `EventDispatcher::fire` runs every mutating-tier listener against the real, live `event: &mut E` exactly as MOD-D39 requires, then runs every Monitor-tier listener against a **fresh clone** of `event`'s own final state (`E: Clone`, a trivial bound for every plain-data event type this catalog defines) — any mutation a Monitor listener makes lands only on that clone, discarded the instant `fire` returns; the real `event` the caller holds is never touched by a Monitor listener, at any point. This is genuine, structural enforcement (verified by `event_cancellation_matrix.rs`'s own dedicated case), not a documented-only promise a careless implementation could quietly violate.

### The M8-alpha event catalog: exactly one entry, `BlockBreakAttempt`

06's own text: "concrete hook enumeration is blueprint-phase work... growing it per-mechanic as `05` lands each system is that document's job" (MOD-D39's own row, mirroring MOD-D8's identical precedent for hooks) — and 06's own Open Questions confirm no procedural workflow yet exists for *who* adds a new event point when a new `05` mechanic lands. This blueprint fixes the **mechanism** (a real, reusable, generic `EventDispatcher<E>`) and ships **one** concrete, real catalog entry — 06's own first-named illustrative example, `BlockBreakAttempt` — rather than inventing a larger catalog with no owning mechanic document behind each entry yet. A future `05-game-mechanics.md`-derived blueprint (M3-B03's own already-merged breaking/placing logic being the natural eventual call site) is this event's own honest, named future integration point — this blueprint does **not** wire `BlockBreakAttempt::fire` into any real block-breaking code path; it ships the event type, the dispatcher, and the full priority/cancellation proof against a synthetic occurrence, exactly mirroring M8-B02/B03's own repeated "prove the mechanism now with a synthetic double, defer the real call site" discipline.

```rust
// crates/mod-api/src/event.rs (new)
use crate::geometry::ModBlockPos;
use crate::registry::ModBlockStateId;

/// MOD-D39's five mutating tiers plus the read-only Monitor tier — six variants,
/// this order, matching 06's own table exactly (Highest first, Monitor always last).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventPriority { Highest, High, Normal, Low, Lowest, Monitor }

/// 06's own first-named illustrative event: one block-break-attempt occurrence.
/// `Clone` is required by `EventDispatcher::fire`'s own Monitor-tier isolation
/// mechanism (Context, above) — trivial for this plain-data shape.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "native-tier", stabby::stabby)]
pub struct BlockBreakAttempt {
    pos: ModBlockPos,
    player_entity: u64,
    block_state: ModBlockStateId,
    cancelled: bool,
}

impl BlockBreakAttempt {
    pub fn new(pos: ModBlockPos, player_entity: u64, block_state: ModBlockStateId) -> Self;
    pub fn pos(&self) -> ModBlockPos;
    pub fn player_entity(&self) -> u64;
    pub fn block_state(&self) -> ModBlockStateId;
    /// The occurrence's own default effect (breaking the block) is vetoed. Callable
    /// by any non-monitor listener, any number of times, at any tier (MOD-D39: "any
    /// non-monitor listener may cancel").
    pub fn cancel(&mut self);
    /// Reverses a previous `cancel()` call (MOD-D39: "may reverse... that outcome").
    pub fn uncancel(&mut self);
    pub fn is_cancelled(&self) -> bool;
}
```

### `EventDispatcher<E>` — the generic, reusable priority/cancellation engine

```rust
// crates/mod-api/src/event.rs (continued), unconditional — pure data/closures, no
// stabby/bevy_ecs dependency; usable directly by native engine code today and by a
// future rc-mod-host bridge (mirroring native_mod_hook_invoke's own role) without
// modification.
pub struct EventDispatcher<E: Clone> {
    /// Index 0..=4: the five mutating tiers, `EventPriority`'s own declared order.
    /// Each `Vec` preserves registration order (FIFO within tier, MOD-D39).
    tiers: [Vec<Box<dyn Fn(&mut E) + Send + Sync>>; 5],
    monitors: Vec<Box<dyn Fn(&mut E) + Send + Sync>>,
}

impl<E: Clone> EventDispatcher<E> {
    pub fn new() -> Self;
    /// Registers `listener` at `priority`. `EventPriority::Monitor` routes into the
    /// dedicated `monitors` list; the other five route into `tiers` by declared
    /// order (`Highest` = 0 .. `Lowest` = 4).
    pub fn register(&mut self, priority: EventPriority, listener: Box<dyn Fn(&mut E) + Send + Sync>);
    /// Dispatches every mutating-tier listener against `event` in place, `Highest`
    /// through `Lowest`, FIFO within each tier — every listener runs regardless of
    /// `event`'s own current cancellation state (MOD-D39: "every remaining
    /// listener... still runs"). Then dispatches every Monitor-tier listener
    /// against a **clone** of `event`'s final state (Context: "Enforcing... cannot
    /// itself cancel or mutate") — `event` itself is never touched during this
    /// second phase.
    pub fn fire(&self, event: &mut E);
    /// Diagnostic/test use — total registered listener count across every tier.
    pub fn listener_count(&self) -> usize;
}
```

### The native-tier mod-facing surface

```rust
// crates/mod-api/src/event.rs (continued), native-tier feature
#[stabby::stabby]
pub trait ModEventListener: Send + Sync {
    /// Uniform signature regardless of registered tier (Context: "Enforcing...
    /// mechanically" — the tier-dependent read-only guarantee is `EventDispatcher`'s
    /// own job, not this trait's).
    fn on_block_break_attempt(&self, event: &mut BlockBreakAttempt);
}
```

`RegistryBuildContext` (M8-B01) gains one new recording method, mirroring `register_channel`'s own shape exactly:

```rust
// crates/mod-api/src/entrypoint.rs (modify — additive only)
impl RegistryBuildContext {
    pub fn register_block_break_attempt_listener(&mut self, priority: EventPriority, listener: stabby::dynptr!(stabby::boxed::Box<dyn ModEventListener>));
}
```

`RecordedRegistrations` gains `pub event_listeners: Vec<(EventPriority, stabby::dynptr!(stabby::boxed::Box<dyn ModEventListener>))>`. A future `rc-mod-host`/composition-root blueprint drains this list across every loaded mod (in MOD-D31's resolved load order, matching every other recorded-registration translation this project already established) and, for each entry, calls `dispatcher.register(priority, Box::new(move |e: &mut BlockBreakAttempt| listener.on_block_break_attempt(e)))` — a trivial adapter closure, not built by this blueprint (no loaded mod exists for it to drain yet, matching `M8-B06a`'s own identical "a future orchestrator" deferral).

### MOD-D41, restated exactly, and the encoding this blueprint fixes

"A reserved `ModComponents` NBT side-channel, byte-exact by construction, never silently dropped; uninstalling a mod never deletes its data." Every entry is keyed `<namespace>:<component>` holding that component's exact raw bytes plus a schema-version integer the mod author owns. Raw byte-exact serialization needs no per-component (de)serialize function because MOD-D13's POD constraint already guarantees round-trip validity. Version-mismatch and uninstalled-mod data are both preserved, never migrated, never dropped — "stays on disk byte-for-byte, invisible to any live query this run."

**What `03-world-chunks-persistence.md` still owns (06's own Interfaces section, restated).** *Where* the `ModComponents` compound lives inside a chunk-column/block-entity/entity NBT record, and whether the fast `ChunkSnapshot` migration path (`WORLD-D20`) carries it losslessly too, are both still-open ratifications 06 flags to `03` — unresolved by this blueprint. What this blueprint fixes, concretely and completely, is the **entry encoding** inside that compound — a self-contained contract any future `03`-owned integration can consume unmodified, exactly as MOD-D41's own rationale already frames it ("this blueprint fixes the schema extension `03`'s NBT (de)serialization path must reserve and pass through untouched").

```rust
// crates/mod-api/src/persistence.rs (new, unconditional)
use crate::Identifier;

/// One mod component's own persisted entry (MOD-D41).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModComponentEntry {
    pub component: Identifier,
    pub schema_version: u32,
    pub raw_bytes: Vec<u8>,
}

/// The reserved `ModComponents` compound's own decoded contents, in insertion order
/// (never reordered — MOD-D41's byte-exact guarantee extends to entry order, so a
/// decode-then-encode round trip is bit-for-bit stable with no entry ever touched).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModComponentsTag { pub entries: Vec<ModComponentEntry> }

impl ModComponentsTag {
    pub fn new() -> Self;
    /// Inserts, or replaces in place (matched by `component`, preserving that
    /// entry's original position), one entry.
    pub fn set(&mut self, entry: ModComponentEntry);
    pub fn get(&self, component: &Identifier) -> Option<&ModComponentEntry>;
    /// Every entry `is_live` accepts, by `(component, schema_version)` — the caller
    /// (a future live-query path) supplies the "is this namespace currently loaded,
    /// at a version I accept" policy; this type has no fixed comparison rule of its
    /// own (MOD-D41: the version integer is "a mod author owns and increments," not
    /// an engine-interpreted value). Never mutates `self` — a dormant entry stays in
    /// `entries` regardless of what any `live_entries` call ever returns, so a
    /// subsequent `encode_mod_components(&self)` always round-trips every entry,
    /// live or dormant (MOD-D41: "never deletes or migrates... it stays... byte-
    /// for-byte").
    pub fn live_entries<'a>(&'a self, is_live: impl Fn(&Identifier, u32) -> bool) -> Vec<&'a ModComponentEntry>;
}

/// Encoding (this blueprint's own binding resolution of MOD-D41's *shape*, not
/// pinned by `06` itself beyond "byte-exact"): each entry is
/// `[u16 le: component-identifier UTF-8 byte length][component-identifier UTF-8
/// bytes][u32 le: schema_version][u32 le: raw_bytes length][raw_bytes]`, entries
/// concatenated in `entries`' own order, no separator, no padding. Chosen because
/// MOD-D13's POD/byte-exact guarantee already makes per-component (de)serialization
/// unnecessary — this format's only job is delimiting *entries* within the
/// compound, never interpreting a component's own bytes.
pub fn encode_mod_components(tag: &ModComponentsTag) -> Vec<u8>;

/// Decodes bytes produced by `encode_mod_components`. Never fails on an unknown
/// namespace or a version this caller doesn't recognize (WORLD-D16's "refuse
/// rather than silently misinterpret," extended per-entry by MOD-D41) — only a
/// genuinely malformed byte stream is `Err`; a syntactically well-formed entry
/// decodes successfully regardless of what it means to the caller.
pub fn decode_mod_components(bytes: &[u8]) -> Result<ModComponentsTag, ModComponentsDecodeError>;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ModComponentsDecodeError {
    #[error("ModComponents byte stream truncated at offset {offset}: expected at least {expected} more bytes, found {found}")]
    Truncated { offset: usize, expected: usize, found: usize },
}
```

### MOD-D42, restated exactly, and the two resolution functions this blueprint adds

"World-query resolution for the chunk entity at a `ChunkKey`, and the block entity at a `BlockPos`." MOD-D6/D13 already let a mod define a component and attach it to any `bevy_ecs::Entity` — this decision names the one missing step: obtaining that `Entity` for a vanilla chunk or block position in the first place. `bevy_ecs::Entity` is not itself ABI-stable (the identical problem M8-B01's `ModComponentId(u64)` already solved for `ComponentId` — Context, M8-B01's own "`ComponentId`-across-ABI rule"); this blueprint mirrors that exact precedent.

```rust
// crates/mod-api/src/world_query.rs (new, unconditional for the plain data types;
// resolution functions themselves live in rc-mechanics, Context below)

/// ABI-safe mirror of `bevy_ecs::Entity` (Context: mirrors `ModComponentId`'s own
/// precedent exactly). `rc-mechanics` alone converts between the two — this crate
/// never constructs a real `Entity`, having no `World` access (identical framing to
/// `ModComponentId`'s own doc comment, M8-B01).
#[cfg_attr(feature = "native-tier", stabby::stabby)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModEntityId(pub u64);

/// Mirrors `rc_core::ChunkKey`'s own three fields exactly (`dimension: u16, x: i32,
/// z: i32` — the identical shape `entrypoint.rs`'s pre-existing `ModAddress::Chunk`
/// variant already uses, restated here as its own standalone type since a
/// resolution *parameter* is a plain key, not a general address).
#[cfg_attr(feature = "native-tier", stabby::stabby)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModChunkKey { pub dimension: u16, pub x: i32, pub z: i32 }
```

**Resolution functions — owned by `rc-mechanics`, the only crate with both a real `bevy_ecs::World`/`ChunkIndex`/block-entity index and (via `M8-B06a`'s already-added dependency) `rc_mod_api` in scope**, mirroring `M8-B06a`'s own `mod_behavior_adapter.rs` precedent of resolving an analogous "which crate can actually build this" gap:

```rust
// crates/mechanics/src/mod_world_query.rs (new)
use rc_core::{ChunkKey, DimensionId};
use rc_mod_api::world_query::{ModChunkKey, ModEntityId};

/// MOD-D42, first call: `ChunkKey` -> chunk entity, always present for any loaded
/// chunk (M3-B01's own `ChunkIndex` doc comment, restated: `WORLD-D1`).
pub fn resolve_chunk_entity(index: &rc_mechanics::stage4::ChunkIndex, key: ModChunkKey) -> Option<ModEntityId> {
    let real_key = ChunkKey::new(DimensionId(key.dimension), key.x, key.z);
    index.0.get(&real_key).map(|&entity| ModEntityId(entity_to_u64(entity)))
}

/// MOD-D42, second call: `BlockPos` -> block entity, `None` for an ordinary,
/// non-block-entity position (never an error — by construction, it simply has no
/// entity of its own). **Moderate-confidence flag, re-verify at implementation
/// time:** this blueprint's own read prerequisites do not include M3-B06's full
/// Deliverables; M3-B06's own `BlockEntityHeader` doc comment ("attached to every
/// block-entity `Entity` (M2-B01's `BlockEntityIndex` members)") is the only
/// citation available for the index type's own name/shape — confirm
/// `rc_chunk_storage::BlockEntityIndex`'s exact field/method shape (most likely,
/// by direct analogy with `ChunkIndex`, a `pub struct BlockEntityIndex(pub
/// HashMap<BlockPos, Entity>)`) against M2-B01's own committed Deliverables before
/// writing this function's body; no other signature in this blueprint changes if
/// the exact index type differs, only this one function's own internal lookup.
pub fn resolve_block_entity(index: &rc_chunk_storage::BlockEntityIndex, pos: rc_mod_api::ModBlockPos) -> Option<ModEntityId>;

/// `Entity`'s own stable bit-packing accessor (Context: mirrors `ModComponentId`'s
/// own conversion precedent). **Moderate-confidence flag:** `bevy_ecs::Entity`
/// 0.19.1's exact public index/generation accessor pair should be re-verified
/// against installed docs, mirroring M8-B01's own identical flag for
/// `ComponentId`'s equivalent conversion.
fn entity_to_u64(entity: bevy_ecs::entity::Entity) -> u64;
```

### Scope boundary, restated

This blueprint deliberately does not touch MOD-D43 (region-scoped mod data on a reserved per-region singleton entity) or MOD-D44 (world-scoped mod data piggybacking on `05`'s GameRules mechanism) — neither is named in this task's own component-attachment bullet ("component attachment to vanilla entities at M8 scope"), and both name their own real, separate structural preconditions (MOD-D43's own per-region bootstrap singleton spawn; MOD-D44's dependency on `05`'s still-unbuilt `MECH-D64` global-value mechanism) that would each be their own, later blueprint's job — naming this explicitly here rather than leaving it to be discovered as a silent gap, mirroring `M8-B06a`'s own identical "Where this blueprint's boundary falls" discipline.

### Determinism, cluster compatibility — restated, unchanged

`EventDispatcher::fire` is ordinary synchronous, in-process dispatch inside whatever system already called it — no new conflict-graph participation, no cross-partition mechanism, matching MOD-D39's own "runs inline... enters no new `ARCH-D8` conflict-graph node" verbatim. `ModComponentsTag`/`encode_mod_components`/`decode_mod_components` are pure, deterministic byte transformations with no I/O of their own. `resolve_chunk_entity`/`resolve_block_entity` read already-region-local state only (`ChunkIndex`/`BlockEntityIndex`, both per-region resources) — neither ever returns an entity from a different region or exposes a `NodeId`/host-identity value (MOD-D15, unchanged).

## Deliverables

### `crates/mod-api/src/event.rs` (new)

`EventPriority`, `BlockBreakAttempt` (+ inherent methods), `EventDispatcher<E: Clone>` (+ inherent methods), `ModEventListener` trait (native-tier), per Context above.

### `crates/mod-api/src/persistence.rs` (new)

`ModComponentEntry`, `ModComponentsTag` (+ inherent methods), `encode_mod_components`, `decode_mod_components`, `ModComponentsDecodeError`, per Context above.

### `crates/mod-api/src/world_query.rs` (new)

`ModEntityId`, `ModChunkKey`, per Context above.

### `crates/mod-api/src/entrypoint.rs` (modify)

`RegistryBuildContext::register_block_break_attempt_listener`; `RecordedRegistrations` gains `event_listeners`.

### `crates/mod-api/src/lib.rs` (modify)

```rust
mod event;
mod persistence;
mod world_query;

pub use event::{BlockBreakAttempt, EventDispatcher, EventPriority};
#[cfg(feature = "native-tier")]
pub use event::ModEventListener;
pub use persistence::{
    ModComponentEntry, ModComponentsDecodeError, ModComponentsTag,
    decode_mod_components, encode_mod_components,
};
pub use world_query::{ModChunkKey, ModEntityId};
// existing entrypoint:: pub use line unchanged apart from the new
// register_block_break_attempt_listener method it now carries
```

### `crates/mechanics/src/mod_world_query.rs` (new)

`resolve_chunk_entity`, `resolve_block_entity`, `entity_to_u64`, per Context above.

### `crates/mechanics/src/lib.rs` (modify)

```rust
mod mod_world_query;
pub use mod_world_query::{resolve_block_entity, resolve_chunk_entity};
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary (TEST-D45/D46), restated exactly, matching `M8-B06a`'s own identical rule.** The test-authoring changeset is every file below plus every new/modified `src/*.rs` file from Deliverables with each function body replaced by `todo!()` (field lists, derives, doc comments stay exactly as specified). The implementation changeset fills in bodies only — it must not modify any file under either crate's `tests/` directory, must not change any type's field list/derive list/public signature, must not weaken any assertion below, and must not touch any pre-existing test file (M8-B01/`M8-B06a`/M3-B01/M3-B06's own) at all.

### `crates/mod-api/tests/event_cancellation_matrix.rs` (pure)

1. `all_five_mutating_tiers_run_in_fixed_order_fifo_within_tier` — register two listeners per mutating tier (10 total), each appending a `(tier, registration_index)` marker to a shared `Rc<RefCell<Vec<_>>>`; `fire` once; assert the recorded sequence is exactly `Highest,Highest,High,High,Normal,Normal,Low,Low,Lowest,Lowest` in registration order within each pair.
2. `highest_tier_cancellation_is_visible_to_every_lower_tier` — a `Highest`-tier listener calls `event.cancel()`; every lower-tier listener (one per remaining mutating tier) records `event.is_cancelled()` at the moment it runs; assert every recorded value is `true`.
3. `a_lower_tier_may_reverse_an_earlier_cancellation` — `Highest` cancels, `Low` calls `uncancel()`; a `Lowest`-tier listener records `is_cancelled()`; assert `false` — MOD-D39's own "may reverse... that outcome."
4. `every_mutating_listener_runs_regardless_of_current_cancellation_state` — `Highest` cancels; assert every one of the four remaining mutating-tier listeners still executed (a shared call counter equals 5 total, not fewer) — MOD-D39's own "every remaining listener... still runs."
5. `monitor_listeners_run_last_and_observe_the_final_cancelled_state` — `Normal` cancels; a `Monitor`-tier listener records `event.is_cancelled()`; assert `true` and assert the monitor's own recorded call happened strictly after every mutating-tier listener's own call (via a shared, monotonically-incrementing call-order counter).
6. `monitor_mutation_is_never_observed_by_the_real_event_or_any_other_listener` — a `Monitor`-tier listener calls `event.uncancel()` on the `&mut BlockBreakAttempt` it receives (the real event was cancelled earlier by a mutating-tier listener); after `fire` returns, assert the caller's own `event.is_cancelled()` is still `true` — the monitor's mutation landed only on `EventDispatcher::fire`'s own discarded clone, mechanically proving Context's "cannot itself cancel or mutate" enforcement.
7. `listener_count_reflects_every_registered_tier_including_monitor` — register one listener per all six `EventPriority` variants; `listener_count() == 6`.
8. `empty_dispatcher_fire_is_a_harmless_no_op` — `EventDispatcher::new().fire(&mut event)` does not panic and leaves `event` unchanged.

### `crates/mod-api/tests/mod_components_persistence.rs` (pure)

1. `single_entry_round_trips_byte_exact` — one `ModComponentEntry` with non-trivial `raw_bytes` (e.g. `[0xDE, 0xAD, 0xBE, 0xEF, 0x00]`, including an embedded NUL to catch any accidental C-string-style truncation bug); `decode_mod_components(&encode_mod_components(&tag))` equals `tag` exactly (`PartialEq`, including entry order).
2. `multi_entry_round_trip_preserves_insertion_order` — three entries with distinct `component` identifiers, non-monotonic insertion order; round trip preserves the exact original `Vec` order.
3. `set_replaces_in_place_preserving_position` — three entries; `set` a new entry for the *middle* one's own `component`; assert the resulting `entries` `Vec` still has that entry at index 1, with the new `raw_bytes`/`schema_version`, and indices 0/2 unchanged.
4. `live_entries_excludes_entries_the_predicate_rejects_but_never_mutates_the_tag` — two entries, one for a "loaded" namespace and one for a "dormant" one; `is_live` returns `true` only for the loaded one; `live_entries` returns exactly one reference; a subsequent `encode_mod_components(&tag)` still round-trips **both** entries — MOD-D41's own "never deletes... it stays... byte-for-byte."
5. `version_mismatched_entry_is_preserved_and_excluded_by_a_version_aware_predicate` — one entry at `schema_version: 2`; `is_live` closure rejects any entry whose version isn't `1`; `live_entries` is empty, but `tag.entries` still contains the one entry unchanged.
6. `truncated_byte_stream_is_a_clean_err_never_a_panic` — three sub-cases: (a) a stream cut off mid-length-prefix, (b) a stream whose declared `raw_bytes` length exceeds the remaining buffer, (c) an empty byte slice for a non-empty expected tag (treated as "zero entries," `Ok(ModComponentsTag::default())` — an empty input is not itself malformed, only a genuinely truncated *non-empty* prefix is); assert (a)/(b) are `Err(ModComponentsDecodeError::Truncated { .. })` and (c) is `Ok` with zero entries.

### `crates/mechanics/tests/mod_world_query.rs` (integration — real `ChunkIndex`/`BlockEntityIndex`)

1. `resolve_chunk_entity_finds_a_present_key` — a real `ChunkIndex` with one inserted `(ChunkKey, Entity)` pair; `resolve_chunk_entity(&index, matching_mod_chunk_key)` is `Some`, and the returned `ModEntityId`'s own `u64` round-trips back to the identical `Entity` via `entity_to_u64`'s own inverse (asserted by comparing against a second, independent `entity_to_u64` call on the same original `Entity`).
2. `resolve_chunk_entity_returns_none_for_an_absent_key` — an empty `ChunkIndex`; `resolve_chunk_entity` is `None`.
3. `resolve_block_entity_finds_a_present_position_and_is_none_for_an_ordinary_block` — a real `BlockEntityIndex` with one inserted `(BlockPos, Entity)` pair; resolution at that exact position is `Some`; resolution at a different, ordinary (non-block-entity) position is `None`.

## Implementation steps

1. **`crates/mod-api/src/event.rs`.** Implement `EventPriority`, `BlockBreakAttempt`, `EventDispatcher<E: Clone>`, `ModEventListener`. Observable: `event_cancellation_matrix.rs` passes in full — no other crate touched, independently verifiable.
2. **`crates/mod-api/src/persistence.rs`.** Implement the encoding per Context's exact byte layout. Observable: `mod_components_persistence.rs` passes in full.
3. **`crates/mod-api/src/world_query.rs`.** Implement `ModEntityId`, `ModChunkKey` (trivial struct literals/derives). Observable: `cargo build -p rc-mod-api --all-features` succeeds.
4. **`crates/mod-api/src/entrypoint.rs` + `lib.rs`.** Add the event-listener registration method and every new `pub use`. Observable: `cargo build -p rc-mod-api --all-features` succeeds with zero warnings; M8-B01/`M8-B06a`'s own full suites still pass unmodified.
5. **`crates/mechanics/src/mod_world_query.rs`.** Resolve the `BlockEntityIndex` moderate-confidence flag against M2-B01's own committed Deliverables first, then implement `resolve_chunk_entity`/`resolve_block_entity`/`entity_to_u64`. Observable: `mod_world_query.rs` passes in full.
6. **`crates/mechanics/src/lib.rs`.** Add the new module/`pub use` lines. Observable: `cargo build -p rc-mechanics` succeeds with zero warnings; M3-B01/M3-B06/`M8-B06a`'s own full suites still pass unmodified.

## Constraints & forbidden actions

(a) The implementation changeset must not modify any file under `crates/mod-api/tests/` or `crates/mechanics/tests/`, must not change any type's field list/derive list/public signature from what the Acceptance tests and Deliverables sections fix, and must not weaken any assertion above. It must not touch any pre-existing M8-B01/`M8-B06a`/M3-B01/M3-B06 test file, fixture, or verification-tooling path (TEST-D46) at all — every edit in this blueprint is additive.

(b) No new external dependency beyond what `M8-B06a` already added to `rc-mechanics` (`rc-mod-api`, `stabby`) and what `rc-mod-api` already carries unconditionally (`serde`, `thiserror`) — this blueprint's own `Cargo.toml` edits are zero lines in both crates.

(c) No Mojang or third-party reimplementation source is consulted or copied — the `ModComponents` byte layout, the `EventDispatcher` clone-and-discard Monitor-tier isolation technique, and the resolution-function shapes are this project's own original design (ASSET-D18/D19/D30).

(d) `EventDispatcher`/`ModComponentsTag`/`resolve_chunk_entity`/`resolve_block_entity` never call `std::panic::catch_unwind` — a real, dylib-loaded native-tier listener's own panic isolation remains entirely `rc-mod-host`'s job (MOD-D32), identically to `M8-B06a`'s own Constraints (d).

(e) `decode_mod_components` never panics on malformed input, by construction (Acceptance test 6's own three sub-cases) — every length prefix is checked against the remaining buffer length before any slice indexing occurs.

## Verification commands

```
cargo nextest run -p rc-mod-api -p rc-mechanics --profile ci
cargo test --doc -p rc-mod-api -p rc-mechanics
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
```

All four commands must run headless and succeed on both `ubuntu-24.04` and `windows-2025` (TEST-D43), producing machine-readable output (TEST-D40). CI tier: Tier 1 (TEST-D37), from a clean checkout (TEST-D50).
