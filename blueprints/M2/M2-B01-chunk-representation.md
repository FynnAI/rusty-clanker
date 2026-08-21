# M2-B01 — In-Memory Chunk Representation: PalettedContainer & Component Decomposition

| Field | Content |
|---|---|
| ID | M2-B01 |
| Milestone | M2 — Persistent World Storage |
| Prerequisites | M0 complete — specifically M0-B01 (workspace scaffold: `crates/chunk-storage/` already exists as an empty-shell crate with `rc-core`/`rc-nbt`/`rc-registries` normal dependencies and the off-by-default `io_uring` feature already wired into its `Cargo.toml`, per `12-workspace-structure.md`'s Crate Manifest), M0-B02 (`rc-core`'s `ChunkKey { dimension: DimensionId, x: i32, z: i32 }`/`DimensionId`/`BlockPos`, reused unmodified — this blueprint imports these types but does not modify `rc-core`), M0-B05 (RC-Executor's ARCH-D8 domain-group/`Access<ComponentId>` conflict model — this blueprint's component decomposition exists specifically to satisfy that model, restated in full below), M0-B07 (`xtask codegen`'s already-committed output, `crates/protocol/generated/v776/{block_states.rs, registries.rs}` — this blueprint reads and aligns with that output's exact layout without depending on it, see Context), M1-B05 (the hand-rolled wire-format encoder in `crates/server/src/play/chunk.rs` this blueprint's own packing algorithm must stay byte-compatible with). Parallel-safe with M2-B02 (on-disk Anvil region-file format and NBT (de)serialization, which consumes this blueprint's types but is not depended on by it — the two blueprints touch disjoint files inside `crates/chunk-storage/src/`). |
| Implements | WORLD-D1 (component decomposition along ARCH-D8 domain boundaries), WORLD-D2 (generic bit-packed `PalettedContainer<T>`, exact palette-state/threshold rules, non-spanning packing), WORLD-D3/D4 (block-state/biome registry integration — restated for this crate's own dependency-constrained reality, see Context's Resolved discrepancy), WORLD-D5 (heightmap types, packing, and the incremental `note_block_change` update rule, to the extent M2 needs it), WORLD-D6 (`BlockEntityIndex`'s storage contract only — no `BlockEntityCodec` implementations, those are `05-game-mechanics.md`'s job), WORLD-D8 (`LightColumn`'s data structure, stored-only, no propagation engine), WORLD-D14 (world height/section-count facts this blueprint's constants restate), WORLD-D22/D23 (`ChunkStatus`'s storage slot and `ChunkPersistenceState`'s dirty/last-saved-tick fields — storage only, no load/save pipeline) |
| Crates touched | `rc-chunk-storage` (`crates/chunk-storage/`) only |
| Estimated scope | L |

## Goal & Done definition

Implement `rc-chunk-storage`'s in-memory chunk representation: a generic, registry-agnostic, non-spanning bit-packed `PalettedContainer<T>` (byte-compatible with the wire encoding `M1-B05` already hand-rolled for its superflat placeholder); the seven `bevy_ecs` components WORLD-D1 decomposes a chunk column into (`BlockStateColumn`, `BiomeColumn`, `LightColumn`, `HeightmapSet`, `BlockEntityIndex`, `ChunkStatus`, `ChunkPersistenceState`) plus the `ChunkKeyTag` identity component; get/set access with automatic palette-strategy upgrades; section iteration; and the dirty-tracking primitives `03-world-chunks-persistence.md`'s save pipeline needs. No NBT (de)serialization, no Anvil region-file I/O, no light propagation, and no world-generation content exist in this blueprint — every one of those is a separate, later blueprint's job (M2-B02 for persistence; `04-worldgen-parity.md`'s own milestone for generation; a future mechanics blueprint for light propagation).

Done when:

- [ ] `cargo build -p rc-chunk-storage --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-chunk-storage`.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint adds exactly one new normal dependency, `bevy_ecs` (already workspace-pinned), to `rc-chunk-storage`; `rc-chunk-storage` is in neither `SIM` nor `NETRENDER` and this addition creates no new edge into either set (see Context's Resolved discrepancy for why this blueprint deliberately does **not** add `rc-protocol` as a dependency here).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-chunk-storage` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### World height and section count (WORLD-D14, restated as constants)

The pinned target's overworld height (from the pinned DataVersion's own `dimension_type/overworld.json`, confirmed by `docs/research/mc-26.2/03-world-chunks.md` §3.17 and reused unchanged by `M1-B05`): `min_y = -64`, `height = 384`, giving **24** 16-block sections per column (`y ∈ [-64, 320)`, section index `0` = `y ∈ [-64, -48)`, section index `23` = `y ∈ [304, 320)`). `LightColumn`'s own section count is `24 + 2 = 26` (WORLD-D8's padding — one section below the lowest real one, one above the highest).

### `PalettedContainer<T>` (WORLD-D2) — exact shape, restated field-precise

Three palette states, non-spanning bit packing, and exact threshold rules — copied verbatim from WORLD-D2 and cross-checked against `M1-B05`'s own already-implemented (and byte-compatible) hand-rolled encoder:

```rust
pub enum Palette<T> {
    SingleValue(T),
    Indirect { entries: Vec<T>, bits_per_entry: u8 },
    Direct { bits_per_entry: u16 },
}
```

Threshold rule, exact: let `distinct` be the palette's current count of distinct values.
- `distinct == 1` → `SingleValue` (zero data words stored — no `bits_per_entry` applies at all).
- `distinct >= 2` → `bits = max(indirect_floor_bits, ceil_log2(distinct))`; `bits <= max_indirect_bits` → `Indirect` at that width; else → `Direct` at a fixed `direct_bits` (the whole target registry's own `ceil(log2(registry_size))`, **not** a function of this container's own distinct-value count — WORLD-D2's own text: "bits per entry fixed by the whole registry's own size").

Two threshold profiles, both restated exactly from WORLD-D2: **blocks** — `indirect_floor_bits = 4`, `max_indirect_bits = 8` (so `Indirect` covers 2..256 distinct values at bit widths 4..8; `Direct` above that). **Biomes** — `indirect_floor_bits = 1`, `max_indirect_bits = 3` (so `Indirect` covers 2..8 distinct values at bit widths 1..3; `Direct` above that, `1` bit minimum instead of `4` since biomes never need `SingleValue`'s "0 bits" floor bumped as aggressively). `direct_bits` itself is never hardcoded by this crate (see Context's Resolved discrepancy below for why) — every constructor takes it as an explicit parameter.

**Non-spanning bit packing**, WORLD-D2's own phrase, the exact algorithm (identical to `M1-B05`'s own `pack_bits`, restated here as this crate's canonical, reusable implementation): pack `entries_per_long = 64 / bits_per_entry` values into each `u64` word, least-significant-bits-first; once a word holds `entries_per_long` values, start a fresh word — **never** split one value's bits across two words, even if that leaves unused high bits in the word just filled. `bits_per_entry == 0` (the `SingleValue` state) stores zero words.

### Registry-id integration (WORLD-D3/D4) — Resolved discrepancy, binding for this blueprint

WORLD-D3/D4's own text describes generated `BlockStateId(u32)`/`BiomeId(u16)` newtypes living at `crates/world/generated/<data-version>/{block_states.rs, biomes.rs}`. Neither path nor crate exists: `12-workspace-structure.md`'s Crate Manifest (the file-owning decision for crate layout, WS-D2) instead assigns generated registry tables to `rc-registries` at `crates/registries/generated/<protocol-version>/` — and flags, in its own Open Questions, that this is **not yet reconciled** against what `M0-B07` (already merged, a Prerequisite of this blueprint) actually built: real `BlockStateId(u32)`/`RegistryEntryId(u32)` generated tables committed at `crates/protocol/generated/v776/{block_states.rs, registries.rs}`, wired into `rc-protocol`'s own module tree by `M1-B05`. `crates/registries/generated/` remains an empty `.gitkeep` placeholder to this day — no blueprint has ever generated content there.

This matters concretely for `rc-chunk-storage`: `xtask lint-deps`'s CI-enforced Rule 2 (`12`'s WS-D3, restated by `M0-B01`) forbids any dependency edge, direct or transitive, between `SIM = [rc-scheduler, rc-mechanics]` and `NETRENDER = [rc-render, rc-protocol, rc-transport-inproc, rc-transport-net, rc-auth, rc-cluster, rc-proxy]`. `rc-mechanics` (in `SIM`) already depends on `rc-chunk-storage` (`M0-B01`'s edge table, via its `server-systems` feature). If `rc-chunk-storage` depended on `rc-protocol` (in `NETRENDER`) to reuse its generated `BlockStateId`/`RegistryEntryId` types directly, the resulting transitive edge `rc-mechanics -> rc-chunk-storage -> rc-protocol` would be a hard CI failure. `rc-chunk-storage` therefore **cannot** use `rc_protocol::generated_v776`'s types directly — not a style choice, a dependency-graph impossibility given the currently-shipped reality.

**Resolution, binding for this blueprint:** `rc-chunk-storage` defines its own minimal, local `RegistryId` trait and two concrete newtypes, `BlockStateId(pub u32)` and `BiomeId(pub u16)` — exactly WORLD-D3/D4's own originally-specified shapes, just homed in this crate instead of a `crates/world/` that was never scaffolded. Every value is **numerically identical** to Mojang's own global registry-assigned id (WORLD-D3's own rationale: "reusing Mojang's own global IDs verbatim... makes... bit-compatible with vanilla with zero translation layer" — this crate's newtypes uphold that identical numeric contract) but is a **textually distinct Rust type** from `rc_protocol::generated_v776::block_states::BlockStateId` / `...::registries::RegistryEntryId`. Bridging the two — converting a `rc_chunk_storage::BlockStateId` to/from the wire-facing `rc_protocol` type — is a thin, free (`#[inline]`, single-field-copy) conversion that belongs in whichever future crate legitimately depends on **both** (e.g. `rc-worldgen`, already depending on `rc-chunk-storage` and free to add `rc-protocol` since it is in neither `SIM` nor `NETRENDER`; or a composition-root binary) — not implemented, and not needed, by this blueprint. This is restated as a standing constraint below (Constraints (d)/(e)).

### `HeightmapSet` (WORLD-D5) — exact packing and update rule, to the extent M2 needs it

Six vanilla types: `WorldSurface`, `WorldSurfaceWg`, `OceanFloor`, `OceanFloorWg`, `MotionBlocking`, `MotionBlockingNoLeaves`. Each a 256-entry (16×16 column) array, packed at `bits = ceil(log2(world_height + 2)) = ceil(log2(386)) = 9` bits/entry (WORLD-D5's own worked value, confirmed by `M1-B05`'s identical heightmap writer), using the **same non-spanning packing** this blueprint's `pack_bits` already implements — a heightmap has no palette, just raw packed integers, each holding `first_available_y - WORLD_MIN_Y` (vanilla's own "first air/free Y from the top, offset from the world floor" convention). `entries_per_long = 64 / 9 = 7`; `256` entries need `ceil(256 / 7) = 37` words (`7 * 9 = 63` bits used per word, 1 bit unused — matches WORLD-D5's own worked value exactly).

Update rule (`docs/research/mc-26.2/03-world-chunks.md` §3.12, restated as this blueprint's binding algorithm — WORLD-D5 explicitly scopes this crate to own the hook's *implementation*, with `05-game-mechanics.md` owning only the *call site*): a same-or-higher opaque placement is an O(1) raise (just overwrite this column's stored value if the new `y` is `>=` the current recorded height); a removal exactly at the current recorded height that turns non-opaque triggers a downward rescan (this crate cannot itself inspect neighboring blocks' opacity — the caller supplies a `column_opacity_below` callback for exactly this rare case); anything strictly below the current recorded height is a guaranteed no-op. Each of the six types has its own opacity predicate (research doc's own table) that only the caller (05, which owns block-property data) can evaluate — `WorldSurface`/`WorldSurfaceWg` use "not air"; `OceanFloor`/`OceanFloorWg`/`MotionBlocking` use "blocks motion" (vanilla additionally folds "non-empty fluid" into `MotionBlocking`'s predicate specifically — the caller's own `blocks_motion` boolean is expected to already account for that when evaluating `MotionBlocking`, since only the caller has fluid-state knowledge); `MotionBlockingNoLeaves` is "blocks motion AND not leaves." This crate never inspects a `BlockStateId`'s own properties — every opacity boolean below is caller-supplied.

**Documented, bounded simplification:** vanilla freezes each `_Wg` variant once worldgen finishes (research doc's own table: "Kept after worldgen? no" for `WorldSurfaceWg`/`OceanFloorWg`, vs. "yes" for their "final" counterparts) — post-worldgen block changes update only the "final" type, leaving the `_Wg` snapshot stale on purpose. This blueprint's `note_block_change` does **not** implement that freeze: it updates a shared predicate's `_Wg` and "final" heightmap in lockstep, always. This is safe and inconsequential at M2's own scope specifically because real world generation does not exist yet (`04-worldgen-parity.md` is `M5`) — there is no consumer anywhere in this milestone that reads a `_Wg` value expecting it to be frozen, since nothing produces the "worldgen just finished" event this rule keys off in the first place. Whichever future blueprint first implements real worldgen (and therefore needs the freeze-after-worldgen behavior to be correct) must add that distinction to `note_block_change`'s call sites, not to this crate's own always-both-together update rule — restated as a standing constraint below (Constraints (g)).

### `LightColumn` (WORLD-D8) — stored data only, no propagation engine

`LightColumn { sections: Vec<LightSection> }`, `LightSection { sky: Option<Box<[u8; 2048]>>, block: Option<Box<[u8; 2048]>> }` (nibble-packed, `None` = vanilla's own "not yet initialized" shortcut), section count `26` (WORLD-D8's `+2` padding, restated above). **This blueprint implements only the data structure and its plain accessors** — no BFS propagator, no cross-chunk/cross-region seeding, no Stage-8 scheduling integration (WORLD-D7/D9/D10 are explicitly out of scope; a future mechanics blueprint owns them).

### `BlockEntityIndex` (WORLD-D6) — storage contract only

`BlockEntityIndex { entities: Vec<bevy_ecs::entity::Entity> }` — a chunk's own placed-block-entity children, in vanilla's own stable per-chunk load order (ARCH-D17). These are ordinary `bevy_ecs::Entity` handles into the **same** region `World` the chunk entity itself lives in (per ARCH-D5, one region owns exactly one `World`) — not `RcEntityId` (that identifier is for cross-region-stable addressing, ARCH-D24, a different concern). This blueprint stores and orders the list; it implements **no** `BlockEntityCodec`, no NBT (de)serialization, and no block-entity spawning — all `05-game-mechanics.md`'s job per WORLD-D6.

### `ChunkStatus`/`ChunkPersistenceState` (WORLD-D22/D23) — storage slots only

`04-worldgen-parity.md` has not landed (its milestone is `M5`); this blueprint therefore defines only the minimal `ChunkGenStatus` distinction M2's own future load/generate-routing logic needs to exist structurally (WORLD-D22: "found... below the required generation status" vs. "`Status = minecraft:full`") — `Generating` (a single placeholder covering every not-yet-`Full` rung of vanilla's real 12-rung ladder) and `Full`. A future `04` blueprint extends this with the real ladder; this blueprint's own Deliverables and tests never reference an intermediate rung, so that extension is additive, not breaking. `ChunkPersistenceState { dirty: bool, last_saved_tick: u64 }` is WORLD-D23's own literal field pair ("`ChunkPersistenceState.dirty`... AND currently need saving (autosave interval elapsed...)").

### Component decomposition and the ARCH-D8 domain claim (WORLD-D1)

One `bevy_ecs::Entity` per loaded chunk column (a future `rc-scheduler`/`rc-worldgen` blueprint's job to actually spawn — not this blueprint's), tagged with a `ChunkKeyTag` identity component and exactly seven data components, each independently addressable by `bevy_ecs`'s `Access<ComponentId>` machinery so that, per `01-server-architecture.md`'s own already-confirmed resolution (`12-workspace-structure.md`'s Open Questions: "chunk/section data **is** modeled as `bevy_ecs` components... so `rc-chunk-storage` depends on `bevy_ecs` directly"), a Block/Redstone system's `Query<&mut BlockStateColumn>` and a Lighting system's `Query<&mut LightColumn>` never conflict on the same chunk entity — the literal mechanism behind ARCH-D8's five-domain-group concurrency model (`M0-B05`'s own `ComponentAccessSummary`/`compute_waves` machinery is what will eventually schedule real systems against these components; this blueprint does not depend on `rc-scheduler` and does not register any system — it only defines types whose `ComponentId`s are, by construction, pairwise distinct and independently accessible).

**Storage-class choice** (`01-server-architecture.md` explicitly defers this: "the storage-class choice (`Table` vs `SparseSet`...) for any component `03` defines is `03`'s call"). This blueprint's decision: every one of the eight components (`ChunkKeyTag` plus the seven data components) uses `bevy_ecs`'s default **`Table`** storage — none of them is ever added or removed independently of the chunk entity's own spawn/despawn (they are all present together from spawn to despawn, per WORLD-D1's own "decomposed into independent components... No single monolithic chunk data component exists," which describes a fixed co-occurring set, not components that toggle on and off); `SparseSet` only pays for itself when a component is frequently added/removed relative to its owning entity's own lifetime, which is never true here. No `#[component(storage = "SparseSet")]` attribute appears anywhere in this blueprint's Deliverables — plain `#[derive(Component)]` (implicitly `Table`) is used throughout, a deliberate, cited decision, not an oversight.

**`ChunkKeyTag`, not a bare `rc_core::ChunkKey` component.** `rc_core::ChunkKey`'s own field shape (`{dimension, x, z}`) is reused completely unmodified (ARCH-D24), but `rc-core` itself does not, and per `12`'s Crate Manifest should not, depend on `bevy_ecs` — `rc-core` is the workspace's zero-dependency root leaf, depended on by all 27 library crates (`WS-D8`), and `12`'s own Crate Manifest lists only `rc-chunk-storage`, `rc-mod-api`, `rc-scheduler`, `rc-mechanics`, and `rc-render` as documented `bevy_ecs` consumers — giving the universal root leaf an ECS dependency for the sake of one wrapper type in one crate is an unbounded, undocumented blast-radius change this blueprint does not make. `ChunkKeyTag(pub rc_core::ChunkKey)` is therefore a thin, local newtype wrapper, deriving `Component` here in `rc-chunk-storage` instead — the wrapped value's own shape and semantics are exactly ARCH-D24's, untouched.

### Dirty tracking is a deliberate two-step hook, not automatic propagation

`BlockStateColumn::set`/`HeightmapSet::note_block_change` each report (via a returned `bool`) whether a mutation actually occurred, but **neither writes into `ChunkPersistenceState` itself** — doing so would require `BlockStateColumn`'s own mutation path to also declare `&mut ChunkPersistenceState` access, collapsing exactly the disjoint-access separation WORLD-D1 exists to create (a Stage-4 Block/Redstone system would then need write access to a component Stage-9's Chunk-Serialization group also needs to read, reintroducing a cross-domain conflict `compute_waves`, `M0-B05`, would then have to serialize around). Instead, whichever future system performs a block write declares `(&mut BlockStateColumn, &mut ChunkPersistenceState)` access together (one system, two components — perfectly legal under ARCH-D8; only *different concurrent systems* touching the same component conflict) and calls `ChunkPersistenceState::mark_dirty()` itself after observing `set`'s `true` return. This blueprint provides both halves of that hook (`set`'s return value and `mark_dirty`/`mark_saved`) but does not wire them together — that wiring is a future mechanics/persistence blueprint's job.

## Deliverables

### `crates/chunk-storage/Cargo.toml` (modify — add one normal dependency)

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
bevy_ecs = { workspace = true }
io-uring = { workspace = true, optional = true }

[dev-dependencies]
proptest = { workspace = true }

[features]
io_uring = ["dep:io-uring"]
```

(`rc-core`/`rc-nbt`/`rc-registries`/`io-uring`/the `io_uring` feature are `M0-B01`'s existing lines, reproduced for a complete, unambiguous file — this blueprint does not use `rc-nbt` or `rc-registries` in its own code, per Context's Resolved discrepancy, but does not remove them either since a later blueprint, M2-B02, needs `rc-nbt`. `bevy_ecs` and the `proptest` dev-dependency are this blueprint's own additions.)

### `crates/chunk-storage/src/lib.rs`

```rust
//! `rc-chunk-storage` — chunk/section/palette data structures, the on-disk region-file
//! format, save scheduling, and a storage-backend abstraction (`03-world-chunks-
//! persistence.md`). This blueprint (M2-B01) implements the in-memory representation
//! only: `PalettedContainer<T>`, the seven WORLD-D1 chunk components, and their get/set/
//! dirty-tracking API. NBT (de)serialization, Anvil region files, and the storage-backend
//! trait are `M2-B02`'s scope.

mod bits;
mod registry_id;
mod palette;
mod column;
mod light;
mod heightmap;
mod block_entity;
mod status;
mod persistence;
mod chunk_key;

pub use bits::{ceil_log2, pack_bits, unpack_bits};
pub use registry_id::{BiomeId, BlockStateId, PaletteThresholds, RegistryId};
pub use palette::{Palette, PalettedContainer};
pub use column::{
    biome_index, block_index, local_biome_quart_y, local_block_y, section_index_for_y,
    BiomeColumn, BlockStateColumn, SECTION_BIOME_CELLS, SECTION_BLOCKS, SECTION_COUNT,
    WORLD_HEIGHT, WORLD_MIN_Y,
};
pub use light::{LightColumn, LightSection, LIGHT_SECTION_COUNT};
pub use heightmap::{BlockOpacity, HeightmapKind, HeightmapSet};
pub use block_entity::BlockEntityIndex;
pub use status::{ChunkGenStatus, ChunkStatus};
pub use persistence::ChunkPersistenceState;
pub use chunk_key::ChunkKeyTag;
```

### `crates/chunk-storage/src/bits.rs`

```rust
/// `ceil(log2(n))` for `n >= 1`; returns `0` for `n <= 1` (both "no bits needed" cases:
/// zero and one distinct value). Exact, allocation-free formula, identical to the one
/// `M1-B05`'s own hand-rolled encoder already uses (`32 - (n - 1).leading_zeros()` for
/// `n >= 2`), reused here so both crates' palette bit-width decisions can never diverge.
pub const fn ceil_log2(n: u32) -> u32;

/// Non-spanning bit-packs `values` into `u64` words (WORLD-D2/WORLD-D5's shared packing
/// primitive — Context). `entries_per_long = 64 / bits_per_entry` values per word, least-
/// significant-bits-first; a value that would not fully fit in the current word's
/// remaining bits starts the *next* word instead of splitting across the boundary.
/// `bits_per_entry == 0` returns an empty `Box<[]>`. Panics (`debug_assert!`) if any
/// `value >= 2^bits_per_entry` or if `bits_per_entry > 64`.
pub fn pack_bits(values: &[u32], bits_per_entry: u32) -> Box<[u64]>;

/// Inverse of `pack_bits`: unpacks exactly `count` values at `bits_per_entry` from `data`.
/// `data` must hold at least `ceil(count / (64 / bits_per_entry))` words for
/// `bits_per_entry > 0` (debug-asserted); for `bits_per_entry == 0`, returns `count`
/// zeros without reading `data` at all (matches `SingleValue`'s "no data words" shape —
/// callers needing the actual single value read it from the palette, not from this
/// function).
pub fn unpack_bits(data: &[u64], bits_per_entry: u32, count: usize) -> Vec<u32>;

/// Reads one packed slot at `index` (0-based) out of `data`, at `bits_per_entry`. Used
/// internally by `PalettedContainer::get`; exposed publicly since a future persistence
/// blueprint's NBT/wire reader needs the identical single-slot read primitive.
pub fn read_slot(data: &[u64], index: usize, bits_per_entry: u32) -> u32;

/// Writes one packed slot at `index` in place, without touching any other slot in the
/// same word. `data` must already be sized for at least `index + 1` entries at
/// `bits_per_entry` (debug-asserted).
pub fn write_slot(data: &mut [u64], index: usize, value: u32, bits_per_entry: u32);
```

### `crates/chunk-storage/src/registry_id.rs`

```rust
/// A palette-storable global registry identifier. Implemented by this crate's own
/// `BlockStateId`/`BiomeId` (Context's Resolved discrepancy — never by
/// `rc_protocol::generated_v776`'s types directly, which this crate must not depend on).
pub trait RegistryId: Copy + Eq + Ord + std::hash::Hash + std::fmt::Debug + Send + Sync + 'static {
    /// The raw global/protocol id, numerically identical to Mojang's own assigned id
    /// for this entry (WORLD-D3's own "reused verbatim" contract).
    fn to_raw(self) -> u32;
    fn from_raw(raw: u32) -> Self;
}

/// Numerically identical to (but a distinct Rust type from)
/// `rc_protocol::generated_v776::block_states::BlockStateId` — Context's Resolved
/// discrepancy explains why this crate cannot use that type directly.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockStateId(pub u32);

/// Numerically identical to (but a distinct Rust type from, and narrower than —
/// `u16` vs. `u32` — since no biome registry remotely approaches 65536 entries)
/// `rc_protocol::generated_v776::registries::RegistryEntryId` when that type holds a
/// biome-registry value. WORLD-D4's own originally-specified shape.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BiomeId(pub u16);

impl RegistryId for BlockStateId {
    fn to_raw(self) -> u32;
    fn from_raw(raw: u32) -> Self;
}
impl RegistryId for BiomeId {
    fn to_raw(self) -> u32;
    fn from_raw(raw: u32) -> Self;
}

/// The two threshold profiles WORLD-D2 pins (Context), plus the Direct-palette bit
/// width — which this crate never hardcodes (a registry's total entry count is not
/// this crate's data, see Context's Resolved discrepancy) — supplied explicitly by
/// every caller.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PaletteThresholds {
    pub indirect_floor_bits: u8,
    pub max_indirect_bits: u8,
    pub direct_bits: u16,
}

impl PaletteThresholds {
    /// `indirect_floor_bits = 4`, `max_indirect_bits = 8` (WORLD-D2). `direct_bits` is
    /// the caller's own `ceil_log2(<block-state registry's total entry count>)` — for
    /// the pinned DataVersion 4903 target, `docs/research/mc-26.2/03-world-chunks.md`
    /// §3.10/§5 records 32366 block states, i.e. `direct_bits = 15`; this crate does not
    /// bake that number in (it would silently go stale if the registry ever regenerates
    /// with a different count) — pass it explicitly, e.g. `PaletteThresholds::blocks(15)`.
    pub const fn blocks(direct_bits: u16) -> Self;

    /// `indirect_floor_bits = 1`, `max_indirect_bits = 3` (WORLD-D2). `direct_bits` is
    /// the caller's own `ceil_log2(<biome registry's total entry count>)` — this
    /// blueprint does not know or assert the real pinned-version biome count (it is not
    /// recorded anywhere in this project's research corpus at the time of writing);
    /// pass it explicitly once a real generated biome registry is available.
    pub const fn biomes(direct_bits: u16) -> Self;
}
```

### `crates/chunk-storage/src/palette.rs`

```rust
use crate::registry_id::{PaletteThresholds, RegistryId};

/// WORLD-D2's three palette states, illustrated identically in `03-world-chunks-
/// persistence.md` itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Palette<T> {
    SingleValue(T),
    Indirect { entries: Vec<T>, bits_per_entry: u8 },
    Direct { bits_per_entry: u16 },
}

/// The generic paletted container (WORLD-D2). One Rust type, two intended future
/// consumers (disk, WORLD-D2's own text; wire, `02-protocol-networking.md`) — though
/// only this crate's own in-memory/disk-facing consumer is wired up by this blueprint;
/// wire-encoder reuse is a future blueprint's integration (Context's Resolved
/// discrepancy — the wire encoder currently lives, hand-rolled and byte-compatible but
/// not type-shared, in `M1-B05`'s `crates/server/src/play/chunk.rs`).
#[derive(Clone, Debug)]
pub struct PalettedContainer<T: RegistryId> {
    palette: Palette<T>,
    data: Box<[u64]>,
    entry_count: u16,
    thresholds: PaletteThresholds,
}

impl<T: RegistryId> PalettedContainer<T> {
    /// A fresh container, every one of `entry_count` entries equal to `value`
    /// (`SingleValue`, zero data words — WORLD-D2). `entry_count` is `4096` for a
    /// block-state section, `64` for a biome section (Context).
    pub fn new_single(value: T, entry_count: u16, thresholds: PaletteThresholds) -> Self;

    /// Reads the value at `index` (`0..entry_count`). Panics (via ordinary slice
    /// indexing) if `index >= entry_count`.
    pub fn get(&self, index: usize) -> T;

    /// Writes `value` at `index`, upgrading the palette strategy in place if needed
    /// (Implementation steps give the exact algorithm: `SingleValue -> Indirect`,
    /// `SingleValue -> Direct`, `Indirect` growth within itself, `Indirect -> Direct`,
    /// or a same-strategy in-place write). Returns `true` iff the value at `index`
    /// actually changed (Context's dirty-tracking hook).
    pub fn set(&mut self, index: usize, value: T) -> bool;

    /// Read-only view of the current palette state — `Indirect`'s `entries`/
    /// `bits_per_entry`, `Direct`'s `bits_per_entry`, or the single `SingleValue`.
    pub fn palette(&self) -> &Palette<T>;

    /// The current bits-per-entry (`0` for `SingleValue`).
    pub fn bits_per_entry(&self) -> u16;

    pub fn entry_count(&self) -> u16;

    /// The thresholds this container was constructed with (needed by a future
    /// serializer to know the registry's own `direct_bits` when re-deriving a palette
    /// from raw values — not otherwise used by this blueprint).
    pub fn thresholds(&self) -> PaletteThresholds;

    /// Read-only access to the packed data words, exactly as `M1-B05`'s wire encoder
    /// would need to embed them (byte-compatibility — Context).
    pub fn raw_words(&self) -> &[u64];

    /// Iterates every entry's value, `index` ascending `0..entry_count`.
    pub fn iter(&self) -> Box<dyn Iterator<Item = T> + '_>;
}
```

### `crates/chunk-storage/src/column.rs`

```rust
use bevy_ecs::prelude::Component;
use crate::palette::PalettedContainer;
use crate::registry_id::{BiomeId, BlockStateId, PaletteThresholds};

pub const WORLD_MIN_Y: i32 = -64;
pub const WORLD_HEIGHT: i32 = 384;
pub const SECTION_COUNT: usize = 24;
pub const SECTION_BLOCKS: u16 = 4096;
pub const SECTION_BIOME_CELLS: u16 = 64;

/// The 0-based section index containing world-Y `world_y`. Panics
/// (`assert!`, not `debug_assert!` — this crate owns world-bounds validation per
/// Context) if `world_y` falls outside `WORLD_MIN_Y .. WORLD_MIN_Y + WORLD_HEIGHT`.
pub const fn section_index_for_y(world_y: i32) -> usize;

/// `world_y`'s local Y (`0..16`) within its own section.
pub const fn local_block_y(world_y: i32) -> u8;

/// `world_y`'s local biome-quart-Y (`0..4`) within its own section (4 quarts per
/// 16-block section).
pub const fn local_biome_quart_y(world_y: i32) -> u8;

/// Local block-in-section index: `(local_y << 8) | (z << 4) | x` — vanilla's own axis
/// order (`docs/research/mc-26.2/03-world-chunks.md` §3.10: `(y<<4|z)<<4|x`), each of
/// `x`/`z` `0..16`, `local_y` `0..16`. `4096` entries per section.
pub const fn block_index(x: u8, local_y: u8, z: u8) -> usize;

/// Local biome-quart-in-section index, same axis order at 4×4×4 resolution: each of
/// `qx`/`qz` `0..4`, `local_qy` `0..4`. `64` entries per section.
pub const fn biome_index(qx: u8, local_qy: u8, qz: u8) -> usize;

/// One chunk column's block-state data (WORLD-D1): `PalettedContainer<BlockStateId>`
/// per section, `SECTION_COUNT` (`24`) sections, `SECTION_BLOCKS` (`4096`) entries
/// each. Storage class: `Table` (Context).
#[derive(Component, Clone)]
pub struct BlockStateColumn {
    sections: Vec<PalettedContainer<BlockStateId>>,
}

impl BlockStateColumn {
    /// Every section `SingleValue(air)` (WORLD-D2's cheapest state — a freshly loaded
    /// or generated column typically starts here before worldgen/persistence populates
    /// real content). `air`'s raw id is conventionally `0` in Mojang's own registration
    /// order (confirmed by `M0-B07`'s own `registries.json` excerpt: `"minecraft:air":
    /// {"protocol_id": 0}`) but this constructor never assumes that — the caller always
    /// passes the concrete `BlockStateId` to fill with.
    pub fn new(air: BlockStateId, thresholds: PaletteThresholds) -> Self;

    /// Reads the block at absolute `(x, world_y, z)`. `x`/`z` must be `0..16`
    /// (`assert!`); `world_y` must be in world bounds (`section_index_for_y`'s own
    /// assertion).
    pub fn get(&self, x: u8, world_y: i32, z: u8) -> BlockStateId;

    /// Writes the block at absolute `(x, world_y, z)`. Returns `true` iff the value
    /// actually changed (Context's dirty-tracking hook — this method never itself
    /// touches `ChunkPersistenceState`).
    pub fn set(&mut self, x: u8, world_y: i32, z: u8, value: BlockStateId) -> bool;

    pub fn sections(&self) -> &[PalettedContainer<BlockStateId>];
    pub fn sections_mut(&mut self) -> &mut [PalettedContainer<BlockStateId>];
    pub fn section(&self, index: usize) -> &PalettedContainer<BlockStateId>;
    pub fn section_mut(&mut self, index: usize) -> &mut PalettedContainer<BlockStateId>;
}

/// One chunk column's biome data (WORLD-D1): `PalettedContainer<BiomeId>` per section,
/// `SECTION_COUNT` sections, `SECTION_BIOME_CELLS` (`64`) entries each. Storage class:
/// `Table`. Independent of `BlockStateColumn`'s own palette/bit-width state (WORLD-D4).
#[derive(Component, Clone)]
pub struct BiomeColumn {
    sections: Vec<PalettedContainer<BiomeId>>,
}

impl BiomeColumn {
    pub fn new(biome: BiomeId, thresholds: PaletteThresholds) -> Self;
    pub fn get(&self, qx: u8, world_y: i32, qz: u8) -> BiomeId;
    pub fn set(&mut self, qx: u8, world_y: i32, qz: u8, value: BiomeId) -> bool;
    pub fn sections(&self) -> &[PalettedContainer<BiomeId>];
    pub fn sections_mut(&mut self) -> &mut [PalettedContainer<BiomeId>];
    pub fn section(&self, index: usize) -> &PalettedContainer<BiomeId>;
    pub fn section_mut(&mut self, index: usize) -> &mut PalettedContainer<BiomeId>;
}
```

### `crates/chunk-storage/src/light.rs`

```rust
use bevy_ecs::prelude::Component;

/// `SECTION_COUNT + 2` (WORLD-D8's padding — one section below the lowest real block
/// section, one above the highest).
pub const LIGHT_SECTION_COUNT: usize = crate::column::SECTION_COUNT + 2;

/// One 16³ light section's nibble-packed sky/block arrays. `None` = vanilla's own
/// "not yet initialized" shortcut (WORLD-D8).
#[derive(Clone, Debug, Default)]
pub struct LightSection {
    pub sky: Option<Box<[u8; 2048]>>,
    pub block: Option<Box<[u8; 2048]>>,
}

/// Stored light data only (WORLD-D8) — no BFS propagator, no cross-chunk seeding
/// (WORLD-D7/D9/D10 are explicitly out of this blueprint's scope, Context). Storage
/// class: `Table`.
#[derive(Component, Clone)]
pub struct LightColumn {
    sections: Vec<LightSection>,
}

impl LightColumn {
    /// `LIGHT_SECTION_COUNT` sections, every one `LightSection::default()`
    /// (uninitialized).
    pub fn new_uninitialized() -> Self;

    pub fn sections(&self) -> &[LightSection];
    pub fn sections_mut(&mut self) -> &mut [LightSection];
    pub fn section(&self, index: usize) -> &LightSection;
    pub fn section_mut(&mut self, index: usize) -> &mut LightSection;
}
```

### `crates/chunk-storage/src/heightmap.rs`

```rust
use bevy_ecs::prelude::Component;

pub const HEIGHTMAP_BITS_PER_ENTRY: u32 = 9; // ceil(log2(384 + 2)), WORLD-D5
pub const HEIGHTMAP_COLUMN_ENTRIES: usize = 256;
pub const HEIGHTMAP_PACKED_LONGS: usize = 37; // ceil(256 / (64/9)), WORLD-D5

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum HeightmapKind {
    WorldSurface,
    WorldSurfaceWg,
    OceanFloor,
    OceanFloorWg,
    MotionBlocking,
    MotionBlockingNoLeaves,
}

impl HeightmapKind {
    pub const ALL: [HeightmapKind; 6] = [
        HeightmapKind::WorldSurface, HeightmapKind::WorldSurfaceWg,
        HeightmapKind::OceanFloor, HeightmapKind::OceanFloorWg,
        HeightmapKind::MotionBlocking, HeightmapKind::MotionBlockingNoLeaves,
    ];
}

/// One changed block's opacity classification against each of the four *distinct*
/// vanilla predicates (`WorldSurfaceWg` shares `world_surface`'s value;
/// `OceanFloorWg` shares `ocean_floor`'s value — Context's own citation of the
/// research doc's opacity table). Every field is caller-supplied — this crate has no
/// block-property data of its own.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BlockOpacity {
    pub world_surface: bool,
    pub ocean_floor: bool,
    pub motion_blocking: bool,
    pub motion_blocking_no_leaves: bool,
}

/// The six WORLD-D5 heightmap types, one packed 256-entry/37-word array each. Storage
/// class: `Table`.
#[derive(Component, Clone)]
pub struct HeightmapSet {
    world_surface: Box<[i64; HEIGHTMAP_PACKED_LONGS]>,
    world_surface_wg: Box<[i64; HEIGHTMAP_PACKED_LONGS]>,
    ocean_floor: Box<[i64; HEIGHTMAP_PACKED_LONGS]>,
    ocean_floor_wg: Box<[i64; HEIGHTMAP_PACKED_LONGS]>,
    motion_blocking: Box<[i64; HEIGHTMAP_PACKED_LONGS]>,
    motion_blocking_no_leaves: Box<[i64; HEIGHTMAP_PACKED_LONGS]>,
}

impl HeightmapSet {
    /// Every column of all six types set to `first_air_y - WORLD_MIN_Y` (WORLD-D5's own
    /// value convention). Intended for a uniform-height placeholder (mirroring
    /// `M1-B05`'s own flat-world heightmap content) or as a test fixture — a real
    /// worldgen/load path is expected to instead build a `HeightmapSet` incrementally
    /// via `note_block_change` or overwrite it wholesale from persisted NBT (M2-B02).
    pub fn new_uniform(first_air_world_y: i32) -> Self;

    /// This type's stored value at column `(x, z)`: `first_available_y - WORLD_MIN_Y`
    /// (WORLD-D5's own convention — **not** an absolute world Y; callers add
    /// `WORLD_MIN_Y` back to recover it).
    pub fn raw(&self, kind: HeightmapKind, x: u8, z: u8) -> u16;

    /// As `raw`, but returns the absolute world Y (`raw(..) + WORLD_MIN_Y`).
    pub fn world_y(&self, kind: HeightmapKind, x: u8, z: u8) -> i32;

    /// Direct overwrite of one column's raw stored value (bypasses the incremental
    /// update rule — for bulk construction/load paths, e.g. `new_uniform` or a future
    /// NBT reader).
    pub fn set_raw(&mut self, kind: HeightmapKind, x: u8, z: u8, raw_value: u16);

    /// WORLD-D5's incremental hook (Context's exact algorithm): the block at absolute
    /// `(x, world_y, z)` changed opacity from `old` to `new`. `column_opacity_below`
    /// resolves, for the one rare downward-rescan case, this type's own opacity
    /// predicate at a given world-Y strictly below `world_y` in the same `(x, z)`
    /// column (caller-supplied — see Context). Updates all six types in one call.
    pub fn note_block_change(
        &mut self,
        x: u8,
        world_y: i32,
        z: u8,
        old: BlockOpacity,
        new: BlockOpacity,
        column_opacity_below: impl Fn(HeightmapKind, i32) -> bool,
    );
}
```

### `crates/chunk-storage/src/block_entity.rs`

```rust
use bevy_ecs::prelude::{Component, Entity};

/// WORLD-D6's storage contract: a chunk's own placed-block-entity children, in
/// vanilla's own stable per-chunk load order (ARCH-D17). No `BlockEntityCodec`, no NBT
/// (de)serialization — `05-game-mechanics.md`'s job (Context). Storage class: `Table`.
#[derive(Component, Clone, Default)]
pub struct BlockEntityIndex {
    entities: Vec<Entity>,
}

impl BlockEntityIndex {
    pub fn new() -> Self;
    /// Appends `entity` at the end of the load order (the caller is responsible for
    /// vanilla-matching order at the point of insertion — this type only preserves
    /// whatever order it is given).
    pub fn push(&mut self, entity: Entity);
    /// Removes the first occurrence of `entity`, if present, preserving the relative
    /// order of every remaining entry. Returns `true` iff an entry was removed.
    pub fn remove(&mut self, entity: Entity) -> bool;
    pub fn entities(&self) -> &[Entity];
}
```

### `crates/chunk-storage/src/status.rs`

```rust
use bevy_ecs::prelude::Component;

/// A single placeholder covering every not-yet-`Full` rung of vanilla's real 12-rung
/// generation ladder (Context — `04-worldgen-parity.md` has not landed; this is the
/// minimal distinction WORLD-D22's own load/generate routing needs to exist
/// structurally today).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ChunkGenStatus {
    Generating,
    Full,
}

/// WORLD-D1's `ChunkStatus` storage slot — `04` owns every value's meaning, this crate
/// only persists/exposes it. Storage class: `Table`.
#[derive(Component, Copy, Clone, Debug, PartialEq, Eq)]
pub struct ChunkStatus(pub ChunkGenStatus);
```

### `crates/chunk-storage/src/persistence.rs`

```rust
use bevy_ecs::prelude::Component;

/// WORLD-D23's own literal field pair. Storage class: `Table`.
#[derive(Component, Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct ChunkPersistenceState {
    pub dirty: bool,
    pub last_saved_tick: u64,
}

impl ChunkPersistenceState {
    pub fn new() -> Self;
    pub fn mark_dirty(&mut self);
    /// Clears `dirty` and records `tick` as the last-saved tick.
    pub fn mark_saved(&mut self, tick: u64);
}
```

### `crates/chunk-storage/src/chunk_key.rs`

```rust
use bevy_ecs::prelude::Component;

/// The chunk-entity identity tag (WORLD-D1). Wraps `rc_core::ChunkKey` — ARCH-D24's
/// `{dimension, x, z}` shape, completely unmodified — in a local newtype so it can
/// derive `bevy_ecs::component::Component` without adding a `bevy_ecs` dependency to
/// `rc-core` itself (Context). Storage class: `Table`.
#[derive(Component, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ChunkKeyTag(pub rc_core::ChunkKey);
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file listed below, plus `crates/chunk-storage/src/{bits.rs, registry_id.rs, palette.rs, column.rs, light.rs, heightmap.rs, block_entity.rs, status.rs, persistence.rs, chunk_key.rs, lib.rs}` with every function body from the Deliverables signatures replaced with `todo!()` (fields, derives, doc comments stay exactly as specified), plus the `Cargo.toml` edit. The implementation changeset (Implementation steps below) fills in real bodies only; it must not modify any file under `crates/chunk-storage/tests/`, and must not change any type's field list, derive list, or public signature from what the test changeset already compiled against.

### `crates/chunk-storage/tests/bits_packing.rs` — hand-computed vectors

1. `pack_single_word_4_bits` — `pack_bits(&[1, 2, 3], 4)` produces exactly one word: `0x321` (`1 | (2 << 4) | (3 << 8)`), i.e. `pack_bits(&[1,2,3], 4)[0] == 0x321` and `.len() == 1`.
2. `pack_zero_bits_is_empty` — `pack_bits(&[0, 0, 0], 0).len() == 0`.
3. `pack_non_spanning_boundary_at_5_bits` — `entries_per_long = 64 / 5 = 12`. Pack 13 values `[1; 13]` (all `1`) at 5 bits: assert `.len() == 2` (13 values need a second word since only 12 fit in the first — proves non-spanning, since `12 * 5 = 60 <= 64` would otherwise tempt a spanning packer to cram a 13th value's remaining bits into the first word's leftover 4 bits); assert word 0 equals `0x84210842108421` (12 copies of `1` at 5-bit stride: `sum(1 << (5*i) for i in 0..12)`, hand-verified digit-by-digit: bit `5*i` set for `i in 0..12`, spanning bits `0..=55`) and word 1 equals `1` (the 13th value, alone, at bit offset 0 of the fresh word).
4. `unpack_inverts_pack_for_arbitrary_values` — `pack_bits(&[7, 0, 5, 3], 3)` then `unpack_bits(&result, 3, 4) == vec![7, 0, 5, 3]`.
5. `unpack_zero_bits_returns_zeros_without_reading_data` — `unpack_bits(&[], 0, 10) == vec![0u32; 10]` (proves the `bits_per_entry == 0` short-circuit never indexes into an empty `data`).
6. `read_write_slot_round_trip` — `let mut data = pack_bits(&[0u32; 20], 6);` then `write_slot(&mut data, 13, 42, 6);` then `read_slot(&data, 13, 6) == 42` and every other index `!= 13` still reads `0` (proves `write_slot` touches only its own slot's bits, not neighbors sharing the same word).
7. `ceil_log2_known_values` — table-driven: `ceil_log2(0) == 0`, `ceil_log2(1) == 0`, `ceil_log2(2) == 1`, `ceil_log2(3) == 2`, `ceil_log2(4) == 2`, `ceil_log2(16) == 4`, `ceil_log2(17) == 5`, `ceil_log2(256) == 8`, `ceil_log2(257) == 9`, `ceil_log2(32366) == 15` (the real pinned-version block-state count, Context).

### `crates/chunk-storage/tests/palette_transitions.rs` — every strategy-boundary crossing

Uses a small, hand-tractable custom threshold profile for the boundary-crossing tests: `let small = PaletteThresholds { indirect_floor_bits: 1, max_indirect_bits: 2, direct_bits: 4 };` (so `Indirect` covers `2..=4` distinct values at `1..=2` bits, `Direct` triggers at the 5th distinct value) — plus one test using the real `PaletteThresholds::blocks(15)`/`PaletteThresholds::biomes(4)` shape to prove the production thresholds behave identically at the algorithm level. Test values are `BlockStateId`s `0..=10` for readability.

1. `starts_single_value` — `PalettedContainer::new_single(BlockStateId(5), 8, small)`; assert `matches!(container.palette(), Palette::SingleValue(BlockStateId(5)))` and `bits_per_entry() == 0` and `raw_words().is_empty()`; every `get(i)` for `i in 0..8` returns `BlockStateId(5)`.
2. `single_value_set_same_value_is_a_noop_that_stays_single_value` — same container, `set(3, BlockStateId(5))` returns `false`; palette is still `SingleValue`.
3. `single_value_to_indirect_at_floor_bits` — `set(3, BlockStateId(7))` returns `true`; palette is now `Indirect { entries: [BlockStateId(5), BlockStateId(7)], bits_per_entry: 1 }` (2 distinct values, `small`'s floor is 1 bit, `ceil_log2(2) == 1`); `get(3) == BlockStateId(7)`; every other index still `get(i) == BlockStateId(5)`.
4. `indirect_grows_bit_width_within_itself` — continuing from test 3's state: `set(4, BlockStateId(9))` (3rd distinct value; `ceil_log2(3) == 2 > 1`, so `bits_per_entry` must grow to `2`, still `<= max_indirect_bits(2)`, so it stays `Indirect`). Assert palette is `Indirect { entries: [.., BlockStateId(9)], bits_per_entry: 2 }`; assert every previously-set index (`3`, and every untouched index) still reads its correct prior value after the repack (proves the repack-on-resize step preserves existing content, not just the newly-written index).
5. `indirect_promotes_to_direct_past_max_indirect_bits` — continuing: `set(5, BlockStateId(2))` (4th distinct value, still fits at 2 bits, stays Indirect) then `set(6, BlockStateId(11))` (5th distinct value; `ceil_log2(5) == 3 > small.max_indirect_bits(2)` → promotes to `Direct { bits_per_entry: 4 }`). Assert palette is now `Direct { bits_per_entry: 4 }`; assert **every** index (not just the two just-touched ones) reads back its correct raw value, including indices that were never explicitly `set` (still `BlockStateId(5)`, the original single value) — proving the promotion correctly remapped every previously-Indirect local index through the old palette into its raw id before switching representations.
6. `direct_set_never_changes_palette_shape` — continuing: `set(0, BlockStateId(99))`; palette is still `Direct { bits_per_entry: 4 }` (Direct never "runs out" of room — it addresses the whole registry already); `get(0) == BlockStateId(99)`.
7. `single_value_can_jump_straight_to_direct` — a fresh container with `PaletteThresholds { indirect_floor_bits: 8, max_indirect_bits: 8, direct_bits: 10 }` (deliberately zero room in `Indirect` beyond exactly 1 valid bit-width, to force the 2-distinct-value case past `max_indirect_bits` — `ceil_log2(2) == 1 <= 8`, so actually let's instead force it structurally: use `PaletteThresholds { indirect_floor_bits: 9, max_indirect_bits: 8, direct_bits: 10 }`, an intentionally-degenerate profile where the floor already exceeds the ceiling); `new_single(BlockStateId(0), 4, thresholds)`; `set(0, BlockStateId(1))` (2 distinct values, `bits = max(9, 1) = 9 > max_indirect_bits(8)`) → promotes directly to `Direct { bits_per_entry: 10 }`, skipping `Indirect` entirely. Assert the palette is `Direct`, never `Indirect`, and every value reads back correctly.
8. `real_block_and_biome_thresholds_reach_indirect_and_stay_there_for_typical_content` — `PalettedContainer::new_single(BlockStateId(0), 4096, PaletteThresholds::blocks(15))`; set 4 distinct values across a handful of indices (mirroring `M1-B05`'s own superflat section: air/bedrock/dirt/grass, 4 distinct); assert palette is `Indirect { bits_per_entry: 4, .. }` (floor-dominated, matching `M1-B05`'s own asserted `bits_per_entry == 4` for its identical 4-distinct-value section) and `entries.len() == 4`. Repeat analogously for `PaletteThresholds::biomes(4)` with a single biome value (never changed) — assert it stays `SingleValue` (matching `M1-B05`'s own biome container, which is `SingleValue` for its uniform-biome placeholder).

### `crates/chunk-storage/tests/paletted_container_properties.rs` — get/set property tests (proptest)

1. `set_then_get_returns_the_written_value` (`proptest!`) — generate a random `entry_count` in `4..=64`, a random sequence of up to 50 `(index, raw_value)` writes (`index < entry_count`, `raw_value < 2^PaletteThresholds::blocks(15).direct_bits`), apply them in order via `set(index, BlockStateId(raw_value))` on a container started `new_single(BlockStateId(0), entry_count, PaletteThresholds::blocks(15))`; after every write, assert `get(index) == BlockStateId(raw_value)` immediately (proves no write is ever lost or corrupted by a subsequent palette-strategy upgrade).
2. `every_untouched_index_keeps_the_single_value_or_its_last_written_value` (`proptest!`) — same setup; after applying the full write sequence, for every index in `0..entry_count`, assert `get(index)` equals either the original `SingleValue` seed (if never written) or the *last* value written to that specific index in the sequence (a simple in-test `HashMap<usize, u32>` "last write wins" oracle) — the general correctness property every transition-boundary test above only spot-checks.
3. `bits_per_entry_never_exceeds_thresholds_or_shrinks_once_grown` (`proptest!`) — same setup; after every single `set` call, assert `container.bits_per_entry() <= thresholds.direct_bits` and that `bits_per_entry()` is monotonically non-decreasing across the whole sequence (a palette, once upgraded, is never silently downgraded by a further `set` — WORLD-D2's algorithm only ever grows or promotes, never shrinks).

### `crates/chunk-storage/tests/column_indexing_and_access.rs`

1. `block_index_matches_vanilla_axis_order` — `block_index(0, 0, 0) == 0`; `block_index(1, 0, 0) == 1`; `block_index(0, 0, 1) == 16`; `block_index(0, 1, 0) == 256`; `block_index(15, 15, 15) == 4095`.
2. `biome_index_matches_vanilla_axis_order_at_quart_resolution` — `biome_index(0,0,0) == 0`; `biome_index(1,0,0) == 1`; `biome_index(0,0,1) == 4`; `biome_index(0,1,0) == 16`; `biome_index(3,3,3) == 63`.
3. `section_index_for_y_boundaries` — `section_index_for_y(-64) == 0`; `section_index_for_y(-49) == 0`; `section_index_for_y(-48) == 1`; `section_index_for_y(319) == 23`; a call with `world_y == 320` or `world_y == -65` panics (`#[should_panic]`, two separate test functions).
4. `local_block_y_wraps_per_section` — `local_block_y(-64) == 0`; `local_block_y(-49) == 15`; `local_block_y(-48) == 0`; `local_block_y(319) == 15`.
5. `block_state_column_get_set_across_section_boundary` — `BlockStateColumn::new(BlockStateId(0), PaletteThresholds::blocks(15))`; `set(5, -49, 8, BlockStateId(42))` (last Y of section 0) returns `true`; `set(5, -48, 8, BlockStateId(43))` (first Y of section 1) returns `true`; assert `get(5, -49, 8) == BlockStateId(42)` and `get(5, -48, 8) == BlockStateId(43)` (proves the two writes landed in different sections, not aliased); assert `section(0)` and `section(1)` each report exactly one non-`SingleValue`... i.e. assert `section(0).palette()` and `section(1).palette()` are each `Indirect` with exactly 2 entries (the section's original air seed plus the one new value), independent of each other.
6. `biome_column_get_set_across_section_boundary` — analogous to test 5, using `BiomeColumn::new`/`PaletteThresholds::biomes(4)` and quart coordinates.
7. `block_state_column_set_same_value_returns_false` — a fresh column; `set(x, y, z, BlockStateId(0))` (the seed air value, unchanged) returns `false`.

### `crates/chunk-storage/tests/heightmap_updates.rs`

Helper: `fn always_false(_: HeightmapKind, _: i32) -> bool { false }` (a `column_opacity_below` stub for tests that never need the rescan branch).

1. `new_uniform_reports_the_given_height_everywhere` — `HeightmapSet::new_uniform(-59)`; for every `(x, z)` in a spot-checked sample (`(0,0)`, `(15,15)`, `(7,3)`) and every `HeightmapKind`, `world_y(kind, x, z) == -59`.
2. `raise_above_current_height_is_an_o1_update` — starting from `new_uniform(-59)`; `note_block_change(3, 10, 4, BlockOpacity{world_surface:false, ocean_floor:false, motion_blocking:false, motion_blocking_no_leaves:false}, BlockOpacity{world_surface:true, ocean_floor:false, motion_blocking:false, motion_blocking_no_leaves:false}, always_false)` (a new opaque-for-`world_surface`-only block placed at `y=10`, well above the current recorded `-59`); assert `world_y(HeightmapKind::WorldSurface, 3, 4) == 11` (one above the placed block, WORLD-D5's own "first air Y" convention) while the other three distinct-predicate types (`ocean_floor`, `motion_blocking`, `motion_blocking_no_leaves`) are unaffected (`new`'s corresponding fields were `false`) and remain at `-59`.
3. `placement_below_current_height_is_a_no_op` — from `new_uniform(-59)` (recorded raw height `5` for every type, corresponding to world Y `-59`); `note_block_change(3, -64, 4, BlockOpacity{motion_blocking:false, world_surface:false, ocean_floor:false, motion_blocking_no_leaves:false}, BlockOpacity{motion_blocking:true, world_surface:true, ocean_floor:true, motion_blocking_no_leaves:true}, always_false)` (a block placed at the world floor, `y = -64` — its own raise-candidate value `world_y + 1 - WORLD_MIN_Y = 1` is less than the current recorded raw `5`, so this is strictly below the recorded height for every type and must not raise anything); assert every `world_y(kind, 3, 4)` is unchanged (`-59`) for all four distinct predicates.
4. `removal_at_current_height_triggers_rescan` — from `new_uniform(-59)` (recorded raw height `5` for every type, corresponding to world Y `-59`, meaning the highest opaque block is at `y = -60`); `note_block_change(3, -60, 4, BlockOpacity{world_surface:true, ocean_floor:false, motion_blocking:false, motion_blocking_no_leaves:false}, BlockOpacity{world_surface:false, ocean_floor:false, motion_blocking:false, motion_blocking_no_leaves:false}, always_false)` (the block at the current highest point turned non-opaque, and the `column_opacity_below` stub reports every lower `y` as also non-opaque — i.e. an entirely empty column below); assert `world_y(HeightmapKind::WorldSurface, 3, 4) == WORLD_MIN_Y` (rescanned all the way to the world floor, finding nothing — WORLD-D5's own "no opaque block anywhere below" edge case).
5. `removal_at_current_height_with_a_lower_opaque_block_found_by_rescan` — as test 4, but `column_opacity_below` is instead `|kind, y| kind == HeightmapKind::WorldSurface && y == -63` (a single opaque block still present a few blocks below, within world bounds); assert `world_y(HeightmapKind::WorldSurface, 3, 4) == -62` (one above the rescan-found block at `-63`).
6. `set_raw_and_raw_round_trip` — `set_raw(HeightmapKind::MotionBlocking, 2, 2, 100)`; `raw(HeightmapKind::MotionBlocking, 2, 2) == 100` and `world_y(..) == 100 + WORLD_MIN_Y`.
7. `packed_word_count_matches_world_d5` — internal/white-box: after `new_uniform(0)`, the crate-internal packed array length is exactly `HEIGHTMAP_PACKED_LONGS` (`37`) — assertable via the module's own `pack_bits`/`HEIGHTMAP_PACKED_LONGS` constant cross-check: `pack_bits(&[5u32; 256], 9).len() == 37`, restated here as a standing regression guard tying `HeightmapSet`'s own internal packing to the shared `bits::pack_bits` primitive.

### `crates/chunk-storage/tests/light_and_index_components.rs`

1. `light_column_has_26_sections_all_uninitialized` — `LightColumn::new_uninitialized().sections().len() == LIGHT_SECTION_COUNT` (`26`); every section's `sky`/`block` are `None`.
2. `light_column_section_mut_round_trips` — `section_mut(0).sky = Some(Box::new([0xFF; 2048]));` then `section(0).sky.as_deref() == Some(&[0xFFu8; 2048])`.
3. `block_entity_index_preserves_push_order` — three distinct `Entity` values (via `bevy_ecs::world::World::spawn(()).id()` three times on a scratch `World`, or `Entity::from_raw(n)` if that constructor is available in the pinned `bevy_ecs` version — implementer confirms against installed docs); `push` all three in order; `entities()` returns them in the same order.
4. `block_entity_index_remove_preserves_relative_order` — four entities pushed in order `[a,b,c,d]`; `remove(b)` returns `true`; `entities() == [a,c,d]`; `remove(b)` again returns `false` (already gone).
5. `chunk_status_default_construction_and_equality` — `ChunkStatus(ChunkGenStatus::Generating) != ChunkStatus(ChunkGenStatus::Full)`; `ChunkStatus(ChunkGenStatus::Full) == ChunkStatus(ChunkGenStatus::Full)`.
6. `chunk_persistence_state_mark_dirty_and_mark_saved` — `ChunkPersistenceState::new()` has `dirty == false`, `last_saved_tick == 0`; `mark_dirty()` sets `dirty == true`; `mark_saved(42)` sets `dirty == false`, `last_saved_tick == 42`.
7. `chunk_key_tag_wraps_rc_core_chunk_key_unmodified` — `ChunkKeyTag(rc_core::ChunkKey::new(rc_core::DimensionId::OVERWORLD, 3, -5)).0 == rc_core::ChunkKey::new(rc_core::DimensionId::OVERWORLD, 3, -5)` (trivial field-preservation proof).

### `crates/chunk-storage/tests/component_access_disjointness.rs` — the ARCH-D8 domain claim

1. `all_eight_components_register_to_distinct_component_ids` — a fresh `bevy_ecs::world::World`; `world.register_component::<ChunkKeyTag>()`, `::<BlockStateColumn>()`, `::<BiomeColumn>()`, `::<LightColumn>()`, `::<HeightmapSet>()`, `::<BlockEntityIndex>()`, `::<ChunkStatus>()`, `::<ChunkPersistenceState>()` — collect all 8 returned `ComponentId`s into a `std::collections::HashSet`; assert `.len() == 8` (proves each component occupies its own independent ECS storage slot — the necessary precondition for any two of them to ever be considered access-disjoint at all).
2. `block_state_and_light_queries_declare_disjoint_write_access` — spawn one entity in a fresh `World` with all eight components attached (a full chunk entity, mirroring how a future worldgen/load blueprint will construct one); build `bevy_ecs::system::QueryState::<&mut BlockStateColumn>::new(&mut world)` and, separately, `QueryState::<&mut LightColumn>::new(&mut world)`; obtain each one's declared `Access<ComponentId>` (the exact method name — `component_access()` or the installed `bevy_ecs` 0.19.1 crate's equivalent — is confirmed against the actually-installed docs at implementation time, mirroring `M0-B05`'s own identically-worded verification note); assert the two access sets are compatible/non-conflicting (via `Access::is_compatible`, if present at the pinned version, or by manually asserting the two `writes()` iterators share no `ComponentId` — either check is acceptable, whichever the installed API surface supports; the assertion's *substance*, not its exact call spelling, is this test's binding requirement). This is the direct, concrete proof of WORLD-D1's own claim: "a Block/Redstone system's declared `Query<&mut BlockStateColumn>` and a Lighting system's declared `Query<&mut LightColumn>` never conflict on the same chunk entity."
3. `block_state_and_persistence_queries_are_also_disjoint` — as test 2, for `QueryState::<&mut BlockStateColumn>` vs. `QueryState::<&mut ChunkPersistenceState>` (the pair Context's "Dirty tracking" subsection explicitly discusses as legitimately co-declarable *by one system*, but must still be recognized as touching independent storage from any *other* system's perspective).

## Implementation steps

1. **`bits.rs`.** `ceil_log2(n)`: `if n <= 1 { 0 } else { 32 - (n - 1).leading_zeros() }`. `pack_bits`: compute `entries_per_long = 64 / bits_per_entry` (return empty immediately if `bits_per_entry == 0`); allocate `ceil(values.len() as f64 / entries_per_long as f64)` (integer ceiling division) `u64` words, zero-initialized; for each `(i, &v)` in `values.iter().enumerate()`, `word = i / entries_per_long`, `slot = i % entries_per_long`, `shift = slot * bits_per_entry`, `data[word] |= (v as u64) << shift`. `unpack_bits`/`read_slot`/`write_slot` are the direct inverse/single-slot forms of the same word/slot/shift arithmetic (`write_slot` additionally masks out the target slot's old bits via `data[word] &= !(mask << shift)` before OR-ing in the new value, where `mask = (1u64 << bits_per_entry) - 1`). Observable: `bits_packing.rs`'s 7 cases pass.
2. **`registry_id.rs`.** `BlockStateId`/`BiomeId`'s `to_raw`/`from_raw` are trivial field access/construction (`BiomeId::to_raw` casts `u16 as u32`; `BiomeId::from_raw` casts `raw as u16`, truncating — safe since no real biome registry remotely approaches 65536 entries). `PaletteThresholds::blocks`/`biomes` are `const fn` struct literals per Deliverables' doc comments. Observable: compiles; no dedicated test file (exercised indirectly by every other test file).
3. **`palette.rs`.** `new_single`: `Self { palette: Palette::SingleValue(value), data: Box::new([]), entry_count, thresholds }`. `get`: match `palette`; `SingleValue(v) => *v`; `Indirect{entries, bits_per_entry} => entries[bits::read_slot(&self.data, index, *bits_per_entry as u32) as usize]`; `Direct{bits_per_entry} => T::from_raw(bits::read_slot(&self.data, index, *bits_per_entry as u32))`. `set`: implement the exact upgrade algorithm from this blueprint's Context ("Dirty tracking..." section is unrelated; the algorithm itself is restated compactly here): (a) `SingleValue(v) if *v == value` → return `false`, no-op. (b) `SingleValue(v)` (different value) → compute `bits = max(thresholds.indirect_floor_bits as u32, ceil_log2(2))`; if `bits <= thresholds.max_indirect_bits as u32`: new palette `Indirect{entries: vec![*v, value], bits_per_entry: bits as u8}`, new `data = pack_bits(&vec![0u32; entry_count as usize] with element `index` set to `1`, rest `0`, bits)` (every entry currently resolves to local index `0` = the old single value, except the just-written `index` which resolves to local index `1` = `value`); else: palette `Direct{bits_per_entry: thresholds.direct_bits}`, `data = pack_bits(&(0..entry_count).map(|i| if i as usize == index { value.to_raw() } else { v.to_raw() }).collect::<Vec<_>>(), thresholds.direct_bits as u32)`. Return `true`. (c) `Indirect{entries, bits_per_entry}`, value already in `entries` at `local`: `write_slot(&mut self.data, index, local as u32, *bits_per_entry as u32)`; return `bits::read_slot(&self.data, index, *bits_per_entry as u32) != local as u32` evaluated *before* the write (capture the old local index first, compare, then write — return whether it changed). (d) `Indirect`, value not present: `new_len = entries.len() + 1`; `new_bits = max(thresholds.indirect_floor_bits as u32, ceil_log2(new_len as u32))`; if `new_bits <= thresholds.max_indirect_bits as u32`: if `new_bits != *bits_per_entry as u32` { unpack every existing local index at the *old* `bits_per_entry`, then `pack_bits` them again at `new_bits` (local index values themselves are unchanged by this repack — only their storage width grows), update `*bits_per_entry = new_bits as u8` and `self.data` to the repacked words }; push `value` onto `entries`; `write_slot(&mut self.data, index, (entries.len() - 1) as u32, new_bits)`; return `true`. Else (promote to Direct): unpack every existing local index at the current `bits_per_entry`, map each through `entries` to its raw id (`entries[local as usize].to_raw()`), overwrite position `index`'s raw value with `value.to_raw()`, set `self.palette = Direct{bits_per_entry: thresholds.direct_bits}`, `self.data = pack_bits(&that_raw_vec, thresholds.direct_bits as u32)`; return `true`. (e) `Direct{bits_per_entry}`: capture old raw via `read_slot`, `write_slot(&mut self.data, index, value.to_raw(), *bits_per_entry as u32)`, return `old_raw != value.to_raw()`. `palette`/`bits_per_entry`/`entry_count`/`thresholds`/`raw_words` are trivial accessors (`bits_per_entry` returns `0` for `SingleValue`, `*bits_per_entry as u16` for `Indirect`, `*bits_per_entry` for `Direct`). `iter`: `Box::new((0..self.entry_count as usize).map(|i| self.get(i)))`. Observable: `palette_transitions.rs`'s 8 cases and `paletted_container_properties.rs`'s 3 proptest cases pass.
4. **`column.rs`.** `section_index_for_y`: `assert!(world_y >= WORLD_MIN_Y && world_y < WORLD_MIN_Y + WORLD_HEIGHT, "world_y {world_y} out of range")`; then `((world_y - WORLD_MIN_Y) / 16) as usize`. `local_block_y`: `((world_y - WORLD_MIN_Y) % 16) as u8` (relies on `world_y >= WORLD_MIN_Y` already having been asserted by a preceding `section_index_for_y` call at every real call site — restate this precondition in a doc comment if not already implied). `local_biome_quart_y`: `local_block_y(world_y) / 4`. `block_index`/`biome_index`: direct bit-shift/OR per Deliverables' doc comments. `BlockStateColumn::new`: `Self { sections: (0..SECTION_COUNT).map(|_| PalettedContainer::new_single(air, SECTION_BLOCKS, thresholds)).collect() }`. `get`/`set`: resolve `section_index_for_y(world_y)` and `block_index(x, local_block_y(world_y), z)`, assert `x < 16 && z < 16`, delegate into `self.sections[section_index].get/set(local_index, value)`. `BiomeColumn` is the direct analogue at quart resolution (`biome_index`, `local_biome_quart_y`, `SECTION_BIOME_CELLS`). Observable: `column_indexing_and_access.rs`'s 7 cases pass.
5. **`light.rs`.** `new_uninitialized`: `Self { sections: (0..LIGHT_SECTION_COUNT).map(|_| LightSection::default()).collect() }`. Accessors are trivial slice/index forwarding. Observable: `light_and_index_components.rs`'s tests 1-2 pass.
6. **`heightmap.rs`.** Internal representation: each of the six fields is `Box<[i64; 37]>` holding the *packed* form directly (not decoded on every access) — `raw`/`world_y` call `bits::unpack_bits`-equivalent single-slot reads (reuse `bits::read_slot` against the field cast as `&[u64]` — `i64`/`u64` share bit patterns; use `.map(|w| w as u64)` or store the fields as `[u64; 37]` internally and only cast to `i64` at any future NBT-write boundary, whichever is simpler; either is acceptable since this blueprint never serializes these fields itself). `new_uniform(first_air_world_y)`: `raw_value = (first_air_world_y - WORLD_MIN_Y) as u32`; build one packed array via `bits::pack_bits(&vec![raw_value; 256], 9)` (converted to a fixed `[i64; 37]`/`[u64;37]`), clone it into all six fields. `set_raw`: `bits::write_slot` at the column's `x*16+z`... **note the heightmap column index is `x + z*16`** (a plain 16×16 row-major XZ index, *not* `block_index`'s Y-major convention — heightmaps have no Y axis to fold in) — restate this explicitly as `fn column_index(x: u8, z: u8) -> usize { x as usize + z as usize * 16 }`, used by every `HeightmapSet` accessor. `note_block_change`: for each of the four distinct predicate fields (`world_surface` driving both `WorldSurface`/`WorldSurfaceWg`, `ocean_floor` driving both `OceanFloor`/`OceanFloorWg`, `motion_blocking` driving only `MotionBlocking`, `motion_blocking_no_leaves` driving only itself) — Context's own predicate-sharing table — run the update rule once per *distinct* field, writing the result into both of a shared field's two kinds where applicable: if `new.<field>` and `world_y + 1 - WORLD_MIN_Y >= current_raw` (a same-or-higher opaque placement — recall stored raw is "first *air* Y", so an opaque block at `world_y` raises the stored value to `world_y + 1 - WORLD_MIN_Y` when that would be higher than the current value; only raise, never lower, on a placement): `set_raw` to `max(current_raw, (world_y + 1 - WORLD_MIN_Y) as u32)`. Else if `old.<field> && !new.<field> && (world_y + 1 - WORLD_MIN_Y) as u32 == current_raw` (removal exactly at the current highest recorded point, turning non-opaque — deliberately phrased as `world_y + 1 - WORLD_MIN_Y == current_raw` rather than `world_y - WORLD_MIN_Y == current_raw - 1`, since `current_raw` is a `u32` that legitimately reaches `0` for an all-air column and `current_raw - 1` would then underflow-panic): rescan strictly downward from `world_y - 1` to `WORLD_MIN_Y`, calling `column_opacity_below(kind, y)` for each candidate `y` (using whichever of the two kinds sharing this predicate field is being updated — pass `HeightmapKind::WorldSurface` when updating the `world_surface` field, etc., per Deliverables' own `column_opacity_below: impl Fn(HeightmapKind, i32) -> bool` signature), stopping at the first `y` where it returns `true` and `set_raw`ing to `(y + 1 - WORLD_MIN_Y) as u32`, or falling all the way through to `WORLD_MIN_Y` (raw `0`) if none is found. Else (placement/removal strictly below current height, or a same-height removal that isn't actually the tracked highest point): no-op for that field. Observable: `heightmap_updates.rs`'s 7 cases pass.
7. **`block_entity.rs`.** `push`: `self.entities.push(entity)`. `remove`: `if let Some(pos) = self.entities.iter().position(|&e| e == entity) { self.entities.remove(pos); true } else { false }` (`Vec::remove`, not `swap_remove` — preserves relative order of the remaining entries, required by the acceptance test). Observable: `light_and_index_components.rs`'s tests 3-4 pass.
8. **`status.rs`, `persistence.rs`, `chunk_key.rs`.** Trivial field/derive-only bodies exactly per Deliverables (`ChunkPersistenceState::new` is `Self::default()`; `mark_dirty` sets `self.dirty = true`; `mark_saved` sets `self.dirty = false; self.last_saved_tick = tick;`). Observable: `light_and_index_components.rs`'s tests 5-7 pass.
9. **`lib.rs`.** Wire the module declarations and `pub use` re-exports exactly as Deliverables. Observable: `cargo build -p rc-chunk-storage` succeeds with zero `todo!()` remaining.
10. **Verify the `bevy_ecs` 0.19.1 API points `component_access.rs`'s test needs** (mirroring `M0-B05`'s own identically-scoped verification note): confirm `bevy_ecs::system::QueryState::<Q>::new(&mut World) -> Self`'s exact constructor signature and the exact accessor name for a `QueryState`'s declared `Access<ComponentId>` (historically `.component_access()`), plus whether `Access<ComponentId>` exposes an `is_compatible` method at this pinned version (if not, write the equivalent manual `writes().collect::<HashSet<_>>()` intersection check instead — Deliverables' test 2/3 doc comments already accept either). Observable: `component_access_disjointness.rs`'s 3 cases pass.
11. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0 (in particular, confirm `lint-deps` still reports zero violations — this blueprint's one new dependency, `bevy_ecs`, is external and touches no `SIM`/`NETRENDER` internal edge).
12. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `crates/chunk-storage/tests/` is committed first, alongside `todo!()`-stubbed `src/*.rs` files (full field lists, full derives, full doc comments) and the `Cargo.toml` edit. The implementation changeset (steps 1-12) fills in real bodies only — it must not edit any test file, must not add, remove, or rename any test case listed in Acceptance tests, and must not weaken any assertion (in particular, `bits_packing.rs`'s hand-computed word values, `palette_transitions.rs`'s exact expected palette shapes at every boundary, and `heightmap_updates.rs`'s exact expected world-Y values must survive unchanged).

(b) **No new external dependencies beyond `bevy_ecs` and `proptest`, both already workspace-pinned.** `bevy_ecs` is this blueprint's one new normal dependency (already in `[workspace.dependencies]` since `M0-B01`, unaltered version). `proptest` is a dev-dependency only, already in `[workspace.dependencies]` since `M0-B02` added it (TEST-D27's pin, `1.11.0`) — reused here, not re-pinned. Do not add `rc-protocol` (Context's Resolved discrepancy — this is a hard, CI-enforced impossibility, not a preference), `rc-scheduler`, `anyhow`, `bitvec`, or any other crate not already present in `rc-chunk-storage`'s `Cargo.toml` from `M0-B01`.

(c) **No Mojang or third-party reimplementation code.** Every fact this blueprint restates (world height/section count, palette threshold values, non-spanning packing, heightmap bit width/update rule, the vanilla axis-order indexing formulas) is sourced from `docs/research/mc-26.2/03-world-chunks.md` and `docs/planning/03-world-chunks-persistence.md`'s own WORLD-D2/D5/D8 (themselves produced under the ASSET-D18/D30 research-role process), cross-checked against `M1-B05`'s own already-merged, byte-compatible encoder — no decompiled source, no third-party reimplementation's code, is consulted or copied while writing any file this blueprint creates.

(d) **Do not add a dependency from `rc-chunk-storage` to `rc-protocol`, `rc-scheduler`, or `rc-mechanics` under any circumstance**, even to "simplify" registry-id handling or component-bootstrap wiring — Context's Resolved discrepancy and `xtask lint-deps` Rule 2 make the first a hard CI failure via `rc-mechanics`'s own existing edge to this crate; the latter two are simply out of this blueprint's layering (this crate is a leaf data-structure crate, never a consumer of the scheduler or mechanics crates that consume *it*).

(e) **Scope boundary — do not implement beyond this blueprint's stated Implements list.** This blueprint does not implement: NBT (de)serialization for any component (`ChunkColumn::to_nbt`/`from_nbt`, WORLD-D11 — `M2-B02`'s scope); the Anvil region-file reader/writer or `ChunkStorageBackend` trait (WORLD-D12/D17 — `M2-B02`'s scope); the light BFS propagator or any Stage-8 scheduling integration (WORLD-D7/D9/D10 — a future mechanics blueprint); the chunk ticket/level system or unload policy (WORLD-D24/D25 — a future `rc-scheduler` blueprint); `BlockEntityCodec` implementations for any concrete block-entity type (WORLD-D6 — `05-game-mechanics.md`); real world-generation content or the real 12-rung `ChunkStatus` ladder (WORLD-D22, `04-worldgen-parity.md`, `M5`); spawning chunk entities into a real region `World` or wiring these components into `RcExecutorBuilder`'s `component_bootstrap` (a future `rc-scheduler`/`rc-worldgen` integration blueprint); any conversion between this crate's `BlockStateId`/`BiomeId` and `rc_protocol::generated_v776`'s equivalents (Context/Constraints (d) — deliberately left for a future blueprint with a legal dependency path to both). Do not add placeholder implementations of any of these as a shortcut — every out-of-scope item stays exactly as unimplemented as this blueprint's Deliverables show it.

(f) **No `unsafe` code.** Every function in this blueprint's deliverables is implementable in 100% safe Rust (bit-packing is ordinary integer arithmetic on `Box<[u64]>`/slices; the `bevy_ecs` `Component` derive and `QueryState` usage in tests are both safe-Rust surface).

(g) **`HeightmapSet::note_block_change`'s known, bounded simplification stands as shipped.** This blueprint's `_Wg`/"final" heightmap pairs (`WorldSurfaceWg`/`WorldSurface`, `OceanFloorWg`/`OceanFloor`) are always updated together, never independently frozen after a worldgen-complete signal (Context's own "Documented, bounded simplification" note) — safe only because no real worldgen exists yet at M2. Do not silently "fix" this by adding a freeze flag or a worldgen-complete parameter to `note_block_change`'s signature in this blueprint — that is real worldgen-integration work belonging to whichever future blueprint first implements `04-worldgen-parity.md`, not this one; changing this crate's signature preemptively without that consumer would be unverifiable guesswork.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43):

```
cargo build -p rc-chunk-storage --all-features
cargo nextest run -p rc-chunk-storage
cargo test --doc -p rc-chunk-storage
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p rc-chunk-storage` runs `bits_packing.rs` (7) + `palette_transitions.rs` (8) + `paletted_container_properties.rs` (3, proptest cases) + `column_indexing_and_access.rs` (7) + `heightmap_updates.rs` (7) + `light_and_index_components.rs` (7) + `component_access_disjointness.rs` (3) = 42 test cases named in Acceptance tests — all pass. CI (`.github/workflows/ci.yml`, `M0-B01`) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
