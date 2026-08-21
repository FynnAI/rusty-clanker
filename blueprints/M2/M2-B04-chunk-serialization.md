# M2-B04 — Chunk NBT Serialization & the Postcard Snapshot

| Field | Content |
|---|---|
| ID | M2-B04 |
| Milestone | M2 — Persistent World Storage |
| Prerequisites | M2-B01 (`rc-chunk-storage`'s in-memory representation — `PalettedContainer<T>`/`bits::{pack_bits,unpack_bits,read_slot}`/the seven WORLD-D1 components + `ChunkKeyTag`, all reused unmodified, none of their field lists, derives, or public signatures altered by this blueprint), M2-B02 (`rc-nbt` — `read_borrowed`/`write_owned`/`SchemaError`/`NbtCompoundExt`/the `borrow`/`owned` module split, consumed as-is). Parallel-safe with any future blueprint that only reads M2-B01/M2-B02's own committed surface without touching `crates/chunk-storage/src/{chunk_nbt.rs,snapshot.rs,lib.rs}` or `crates/chunk-storage/Cargo.toml`. |
| Implements | WORLD-D2 (paletted-container format, restated for the **on-disk** encoding — a distinct rule set from the in-memory/wire form M2-B01 already implements, Context), WORLD-D3/D4 (registry-id integration — the resolver seam this blueprint defines, extending M2-B01's own "Resolved discrepancy"), WORLD-D5 (heightmap NBT schema and the four-of-six persisted-types rule), WORLD-D6 (block-entity storage contract — empty-tolerant enforcement, no codec), WORLD-D8 (light-section-to-NBT-section-Y mapping), WORLD-D11 (hand-written, non-derived NBT conversion), WORLD-D16 (DataVersion 4903, exact-match-required, no DFU), WORLD-D20 (versioned postcard `ChunkSnapshot`), WORLD-D22/D23 (`ChunkStatus`/`ChunkPersistenceState` NBT mapping); TEST-D25–D28 (proptest/fuzz obligations, restated), TEST-D39/D45–D47 (acceptance-test mandate, test-first changeset boundary, fixture-manifest policy — restated, honest gap noted since `rc-golden-data` does not exist yet, Context) |
| Crates touched | `rc-chunk-storage` (`crates/chunk-storage/`) only |
| Estimated scope | L |

## Goal & Done definition

Give `rc-chunk-storage` two new capabilities, built entirely on M2-B01's already-committed components and M2-B02's already-committed `rc-nbt` surface: (1) `chunk_nbt` — hand-written (never derived, WORLD-D11) (de)serialization between the eight M2-B01 chunk components and the vanilla chunk NBT compound schema, at the pinned DataVersion, with an explicit, bounded, documented policy for every vanilla field this milestone does not yet model; (2) `snapshot` — the WORLD-D20 versioned `postcard` `ChunkSnapshot`, the fast in-memory hand-off format Stage 9/cluster migration will serialize through, independent of NBT entirely. No Anvil `.mca` container, no `ChunkStorageBackend` implementation, no async I/O-pool wiring, and no real vanilla schema fields this milestone's components cannot represent (structures, scheduled ticks, inhabited time) are implemented with real semantics — every one of those is out of scope, restated in Constraints.

Done when:

- [ ] `cargo build -p rc-chunk-storage --all-features` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-chunk-storage`.
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — this blueprint adds three new normal dependencies to `rc-chunk-storage` (`postcard`, `serde`, `thiserror`), all already workspace-pinned, all external (no new internal RC-crate edge), so no `SIM`/`NETRENDER` rule is newly touched (restated in Constraints).
- [ ] `cargo run -p xtask -- fmt-check` and `cargo run -p xtask -- lint` both exit 0.
- [ ] `cargo test --doc -p rc-chunk-storage` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`, plus M0-B08's already-existing `path-guard`/`lint-tests`/`verify-fixtures` gates) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### DataVersion policy (WORLD-D16, restated exactly)

The pinned target's DataVersion is **4903** (Java Edition 26.2, matching NET-D1's protocol-776 pin — both derive from the same release). Policy: **exact match required, no upgrade path in Phase 1.** A chunk NBT document whose `DataVersion` field is not exactly `4903` is refused at load with an explicit, actionable error — never silently accepted, never partially migrated. Every document this blueprint's own code writes always stamps `DataVersion = 4903`. This mirrors NET-D2's identical single-version-tracking discipline applied to the wire protocol.

### World geometry constants (restated from M2-B01, unchanged)

`WORLD_MIN_Y = -64`, `WORLD_HEIGHT = 384`, `SECTION_COUNT = 24` (block sections, index `0..24`), `LIGHT_SECTION_COUNT = 26` (`SECTION_COUNT + 2` padding, WORLD-D8). This blueprint additionally fixes `MIN_SECTION_Y: i32 = WORLD_MIN_Y / 16 = -4` — the vanilla `yPos` root field's value and the anchor every section's stored `Y` byte is computed from (`vanilla_y = MIN_SECTION_Y + block_index`, `block_index ∈ 0..24` → `vanilla_y ∈ -4..19`). A loaded document whose `yPos` is not exactly `-4` is refused (`ChunkNbtError::UnexpectedYPos`) — this blueprint's fixed world-height constants have no per-dimension variation yet (Nether/End height differences are a future dimension-aware blueprint's scope, out of M2), so any other `yPos` is currently unrepresentable, not silently truncated.

### Light-section ↔ NBT-section-Y mapping (WORLD-D8, new to this blueprint)

M2-B01's `LightColumn` holds `LIGHT_SECTION_COUNT` (26) sections: index `0` is the padding section *below* the lowest block section, indices `1..25` align one-to-one with block sections `0..24`, index `25` is the padding section *above*. Vanilla's own on-disk schema stores light-only section entries at the same Y coordinates a light-carrying section would occupy if it held blocks — confirmed against `minecraft.wiki`'s Chunk format page (fetched 2026-08-21): a section entry may legally carry only `BlockLight`/`SkyLight` with no `block_states`/`biomes` at all, and "a section with none of `block_states`/`biomes`/light present is dropped from the list entirely" (already restated by `03-world-chunks-persistence.md` itself). This blueprint's binding mapping:

| Light index | Vanilla `Y` | Content |
|---|---|---|
| `0` | `MIN_SECTION_Y - 1` = `-5` | light-only (`BlockLight`/`SkyLight` if present; no `block_states`/`biomes`) — **omitted from `sections` entirely if both are `None`** |
| `1..25` | `MIN_SECTION_Y + (light_index - 1)` = `-4..19` | a real block section: `block_states` + `biomes` always present (M2-B01's columns are always densely populated for all 24 sections), `BlockLight`/`SkyLight` if present |
| `25` | `MIN_SECTION_Y + SECTION_COUNT` = `20` | light-only, same omit rule as index `0` |

The `sections` NBT list is written in ascending `Y` order: the below-padding entry (if present), then the 24 real sections in order, then the above-padding entry (if present) — `24..26` total entries.

### On-disk paletted-container encoding — a *different* rule set from M2-B01's in-memory form (web-verified, thin spot)

M2-B01's `PalettedContainer<T>` models WORLD-D2's **wire-compatible** rule set: `SingleValue` / `Indirect` (bit width capped at `max_indirect_bits`, `8` for blocks) / `Direct` (fixed width = the whole registry's own bit count, **no local palette at all** — WORLD-D2's own text: "full registry ID, no local table"). Vanilla's **on-disk** NBT format does not have an equivalent of that last, palette-less `Direct` state at all — confirmed via a live fetch of `minecraft.wiki`'s Chunk format page (2026-08-21): "the on-disk NBT chunk format always uses an explicit local palette... there is no direct global-registry-id mode without a palette... this array will never contain more than 4096 entries [blocks] ... indices... at most 12 bits long", with the same `4`-bit floor already established for the wire form. Concretely: the on-disk `bits_per_entry` for a section is `0` when exactly one distinct value is present (no `data` array written — "if only one block state is present in the palette, this field is not required"), else `max(floor_bits, ceil_log2(distinct_count))`, **uncapped** above `floor_bits` (blocks can in principle reach 12 bits at up to 4096 distinct values; biomes up to 6 bits at up to 64 distinct values) — never switching to a palette-less mode the way the in-memory/wire `Direct` state does. `floor_bits = 4` for blocks (confirmed by the fetch above) and `floor_bits = 1` for biomes (this blueprint's own inference from WORLD-D2's *wire* floor value, not independently confirmed on-disk by the fetch above — flagged in Open Questions for re-verification against a real vanilla-produced chunk once `rc-test-harness` exists).

**Binding consequence:** this blueprint's NBT encoder never reads `PalettedContainer::palette()`/`bits_per_entry()`/`raw_words()` to decide the disk representation — those reflect the *wire-form* state, which can legitimately disagree with the disk rule above once a section exceeds 256 distinct values (in-memory: promoted to palette-less `Direct`; on disk: still an explicit, wider-bit-width palette). Instead, the encoder always **re-derives** the on-disk palette and packed data fresh from `PalettedContainer::iter()` (already public, M2-B01) — collecting distinct values in first-encountered order via a `HashMap<T, u32>` (value → local index) built in one pass, then `bits::pack_bits`-ing the resulting local-index sequence at the disk-derived width. This is correct for every in-memory palette state (`SingleValue`/`Indirect`/`Direct` alike) with **one** algorithm, and is exercised specifically by this blueprint's `palette entry edge cases` acceptance tests (a section with `>256` distinct values, forcing 9+-bit on-disk packing even though the in-memory container itself sits in wire-`Direct` mode at a completely different, wider fixed width).

### Registry-id resolver seam (WORLD-D3/D4, extending M2-B01's own Resolved discrepancy)

M2-B01's `BlockStateId(u32)`/`BiomeId(u16)` carry no name or property data — they are bare numeric ids, and `rc-chunk-storage` deliberately stays decoupled from `rc-registries`' generated tables (M2-B01's own Context; `xtask lint-deps` Rule 2 separately bars the wire-facing route through `rc-protocol`, restated in Constraints below). Vanilla's NBT palette entries need a block's namespaced **name** and **property key/value strings**, and a biome's namespaced name — data this crate does not own and has no path to obtain from any currently-committed crate (M0-B07's own generated `crates/registries/generated/v776/block_states.rs` emits only each block's single flagged-**default**-state constant, not a full per-state id→{name, properties} table for every one of the pinned target's 32366 states — restated from M0-B07's own Deliverables; no committed crate anywhere in this workspace currently has that full table). This blueprint's binding resolution, in the same spirit as M2-B01's `HeightmapSet::note_block_change`'s caller-supplied `column_opacity_below` closure: two small, crate-owned **resolver traits**, injected by the caller at every (de)serialization call, never implemented by this crate itself:

```rust
pub trait BlockStateNames {
    /// The block's namespaced id and its state's property key/value pairs, in **any**
    /// order — this crate re-sorts them before writing (next subsection). `None` means
    /// "this crate's registry has no entry for `id`" (an incomplete/corrupt resolver,
    /// or a raw id from a newer registry this build does not know about).
    fn name_and_properties(&self, id: BlockStateId) -> Option<(rc_nbt::Mutf8String, Vec<(rc_nbt::Mutf8String, rc_nbt::Mutf8String)>)>;
    /// The inverse: a name + property set (in whatever order the NBT document stored
    /// them) resolved back to a concrete id. `None` if no registered state matches.
    fn resolve(&self, name: &rc_nbt::Mutf8Str, properties: &[(&rc_nbt::Mutf8Str, &rc_nbt::Mutf8Str)]) -> Option<BlockStateId>;
}

pub trait BiomeNames {
    fn name(&self, id: BiomeId) -> Option<rc_nbt::Mutf8String>;
    fn resolve(&self, name: &rc_nbt::Mutf8Str) -> Option<BiomeId>;
}
```

No implementation of either trait ships in this blueprint. A future blueprint with a legal dependency path to both this crate and a real generated per-state registry table (e.g. `rc-worldgen`, or a composition-root binary — the same category M2-B01's Context already names for the `BlockStateId` bridging conversion it also deferred) supplies the real implementation. This blueprint's own tests define a small, synthetic, hand-authored mock registry — not real Mojang data, purely local id↔name test fixtures (ASSET-D18/D19-clean: these are test-only names this blueprint invents, never extracted from `--reports` or a real jar).

### Property-compound ordering (the "palette entry edge cases" requirement, made concrete)

`04-persistence-nbt.md`'s own §8 note, already on record: vanilla's own `CompoundTag` is `HashMap`-backed and does **not** guarantee stable key order across a Java save/reload, and a from-scratch Rust writer choosing a **deterministic** key order instead is an explicitly endorsed, harmless deviation ("stays byte-different from vanilla-written ones but remains semantically... identical"). This blueprint's binding rule: `to_nbt` sorts a `Properties` compound's entries by property-name bytes, ascending, before writing — regardless of the order `BlockStateNames::name_and_properties` returned them in. A block with **zero** properties omits the `Properties` tag from its palette-entry compound entirely (never an empty compound) — consistent with the wiki's own Block State Palette Entry description ("`Properties` — optional").

### Heightmaps — four of the six types persist, the other two are exactly reconstructed on load

Vanilla persists only the four "final" heightmap types (`WORLD_SURFACE`, `OCEAN_FLOOR`, `MOTION_BLOCKING`, `MOTION_BLOCKING_NO_LEAVES`) — confirmed by both the research doc's own table ("Kept after worldgen?" column: `no` for `WORLD_SURFACE_WG`/`OCEAN_FLOOR_WG`) and the live wiki fetch above, which lists all six as *valid* NBT keys but does not claim a `FULL`-status chunk ever writes the two `_WG` ones (vanilla's own `LevelChunk` promotion constructor only copies the four "final" types out of a finishing `ProtoChunk` in the first place, per the research doc's §3.2). This blueprint's `to_nbt` therefore writes exactly these four `LongArray` entries (37 longs each, `bits::pack_bits` at `HEIGHTMAP_BITS_PER_ENTRY = 9` over the 256 `HeightmapSet::raw()` values per type, M2-B01's own already-established packing) and never writes `WORLD_SURFACE_WG`/`OCEAN_FLOOR_WG`.

On load, `from_nbt` reconstructs a full six-field `HeightmapSet` by `set_raw`-ing the four loaded types directly, then setting `WorldSurfaceWg := WorldSurface`'s just-loaded raw value and `OceanFloorWg := OceanFloor`'s, column by column. **This is not a lossy approximation at M2's scope, it is exact**: M2-B01's own `HeightmapSet::note_block_change` (that blueprint's Constraint (g), cited verbatim) already guarantees every `_Wg`/"final" pair stays numerically identical in lockstep for *every* `HeightmapSet` this milestone's own code can ever produce (no real worldgen exists yet to ever legitimately diverge them) — so reconstructing `_Wg := final` on load reproduces exactly the value that was in memory before the save that produced the document being loaded, for any chunk this project can currently build. The round-trip guarantee below is therefore unqualified, not "modulo the `_Wg` fields."

### Block entities — empty-tolerant, not silently dropped (WORLD-D6)

`BlockEntityIndex` (M2-B01) holds only `bevy_ecs::Entity` handles — no NBT data, no `BlockEntityCodec` exists yet (05's job, per WORLD-D6). This blueprint cannot serialize a *non-empty* index at all. Binding policy, both directions: `to_nbt` returns `Err(ChunkNbtError::UnsupportedBlockEntities(n))` if `block_entities.entities()` is non-empty (never silently writes `block_entities: []` while dropping real entries); `from_nbt` returns the same error kind if the loaded `block_entities` list is non-empty (never silently discards real block-entity NBT). The only case this blueprint actually round-trips is the empty one — which is also the *only* case M2's own milestone boundary can produce in practice (full block-placement mechanics, M3, have not landed; block entities cannot yet be placed by anything this engine does).

### Fixed-default fields, and the true "unknown tag" preservation policy (this blueprint's own binding decision — `03` names no prior ruling to copy)

Neither `docs/planning/03-world-chunks-persistence.md` nor its research corpus states an explicit unknown-tag preserve-or-reject policy — this blueprint therefore makes and states that decision itself, rather than inventing a claim of a prior ruling that does not exist. Five vanilla root-level fields are **always present** in a real chunk document (research doc §3.6: "always") but have **no corresponding M2-B01 component** to hold real values (`InhabitedTime`, `structures`, `block_ticks`, `fluid_ticks`, `PostProcessing` — cumulative play-time, structure indices, scheduled ticks, and post-generation deferred work, all 04/05 concerns not yet built). This blueprint's binding, two-part policy:

1. **Fixed-default fields** (the five named above): `to_nbt` **always** writes a fixed, empty/zero placeholder for each — `InhabitedTime: Long(0)`, `structures: {starts: {}, References: {}}`, `block_ticks: []`, `fluid_ticks: []`, `PostProcessing: [<24 empty Short lists>]` — regardless of what a loaded document's own values were. `from_nbt` reads and discards them (validated only for presence and roughly correct tag type — a missing one is *not* a load error, since this blueprint's own fresh-chunk writer output always includes them but a hand-authored test fixture reasonably might omit one; a present-but-wrong-tag-type one *is* rejected as malformed). **This is a named, bounded, documented parity gap**: a real vanilla chunk's inhabited time, scheduled ticks, and structure references are silently reset to empty on every save-then-reload cycle through this crate, until a future 04/05 blueprint gives this crate real components to carry them.
2. **Everything else** — any root-level tag that is neither one of the fields this blueprint actively models (`DataVersion`, `xPos`/`yPos`/`zPos`, `Status`, `LastUpdate`, `isLightOn`, `sections`, `Heightmaps`, `block_entities`) nor one of the five fixed-default fields above (`blending_data`, `below_zero_retrogen`, `UpgradeData`, `carving_mask`, `entities`, or any future/unrecognized tag) — is **preserved opaquely**: `from_nbt` captures every such tag verbatim (name + owned value, via `NbtCompound::iter()` + `.to_owned()`) into `extra: Vec<(Mutf8String, owned::NbtTag)>`, in encounter order; `to_nbt` accepts that same `extra` slice as a parameter and re-emits every entry verbatim, appended after every known and fixed-default field, in its original captured order. Per the "safe, harmless deviation" framing already established (Property-compound ordering, above), this canonicalizes tag order relative to vanilla's own unspecified `HashMap` order — a difference in byte layout, not in decoded meaning.

**Byte-identity guarantee, precisely scoped:** `chunk_from_nbt(chunk_to_nbt(components, extra: &[])) == components` holds **unconditionally**, compared component-by-component over M2-B01's eight components (`extra` and the five fixed-default fields are explicitly outside this guarantee — see policy above). `chunk_to_nbt(chunk_from_nbt(vanilla_bytes).into_parts())`'s `extra` output is identical, tag-for-tag (order-canonicalized, values byte-identical), to whatever genuinely-unrecognized tags `vanilla_bytes` carried — this is the "unknown-field policy: preserve" half of the guarantee; the five fixed-default fields are the one explicit, named exception where re-saving does *not* preserve a loaded real value.

### `Status` mapping (WORLD-D22, restated for M2-B01's placeholder ladder)

M2-B01's `ChunkGenStatus` has exactly two values (`Generating`, `Full` — a deliberate placeholder for vanilla's real 12-rung ladder, per that blueprint's own Context). Binding mapping: `to_nbt` writes `Status: "minecraft:full"` for `Full`, `Status: "minecraft:empty"` for `Generating` (vanilla's own rung-0 name, the most conservative choice — no other rung's semantics are modeled). `from_nbt` maps `"minecraft:full"` → `Full`; **every other string** (including all ten intermediate real vanilla rungs, and any unrecognized value) → `Generating` — a deliberate, documented many-to-one collapse (a real vanilla chunk mid-generation is treated as "not yet done" by this milestone, never rejected outright).

### `isLightOn` is a plain, caller-supplied `bool` — never derived, never stored

Real `isLightOn` semantics depend on WORLD-D7/D9's light BFS propagator, which does not exist yet (out of scope for M2-B01 and this blueprint alike). Rather than inventing an unfounded heuristic, this blueprint treats `isLightOn` as an ordinary `bool` parameter to `to_nbt` and an ordinary `bool` field of `from_nbt`'s return value — the caller (a future lighting-integration blueprint) decides what it means and where, if anywhere, to store it; this crate only guarantees it round-trips exactly (present-only-if-`true`, per vanilla's own convention, restated from `04-persistence-nbt.md` §3.6: "only if `true`... omitted entirely when light is not yet valid").

### The versioned postcard `ChunkSnapshot` (WORLD-D20)

A **separate, parallel** representation from the NBT schema above — used only for Stage-9-adjacent fast in-memory hand-off (cluster migration/takeover staging, per WORLD-D20's own text), never for durable Anvil storage. M2-B01's own component types (`BlockStateColumn`, `PalettedContainer<T>`, `BlockStateId`, etc.) derive no `serde` impls (that blueprint's own Deliverables list confirms this) and cannot be retroactively given any — matching M2-B01's own already-established pattern of defining fresh, locally-owned types rather than modifying a locked prerequisite, `ChunkSnapshot` is its own flat, self-contained `struct` tree, built only from raw scalar data already reachable through M2-B01's fully public accessors (`PalettedContainer::iter()`, `HeightmapSet::raw()`) — never a `#[derive(Serialize)]` wrapping M2-B01's own types directly. `rc_core::ChunkKey`/`DimensionId` **do** already derive `serde::Serialize`/`Deserialize` (confirmed from M0-B02's own committed Deliverables) and are embedded directly.

`format_version: u16` (`RC_CHUNK_SNAPSHOT_VERSION`) is written as a **fixed 2-byte big-endian prefix outside the postcard-encoded body**, never as the struct's own first field — this is what makes `peek_snapshot_version` decodable without knowing any future version's body shape at all, the concrete mechanism behind WORLD-D20's "prefixed with a `format_version`" text. Version policy mirrors WORLD-D16's DataVersion policy exactly, applied to this second, independent versioning axis: **exact match required, no migration** — a mismatched `format_version` is a hard decode error, never silently accepted or partially interpreted. Block entities are **not** captured by `ChunkSnapshot` at M2 (WORLD-D20's own text names "`BlockEntityIndex`'s resolved records," which requires the `BlockEntityRecord`/`BlockEntityCodec` machinery WORLD-D6 defers to `05-game-mechanics.md` — not yet buildable) — a named, bounded gap, restated in Constraints, for a future blueprint once that codec exists.

### Golden fixtures and TEST-D47 — an honest gap, restated from M2-B02's own precedent

No blueprint through M2-B02 has created `crates/testing/rc-golden-data/` or an `xtask golden-export` verb (confirmed: M0-B01 fixes `xtask`'s surface at exactly `fmt-check`/`lint`/`lint-deps`/`test`; M0-B08 wires `path-guard`/`lint-tests`/`verify-fixtures` against a fixture manifest path that "currently matches zero files"; no later blueprint through M2-B02 adds either). This blueprint therefore cannot register any fixture in a TEST-D47 manifest that does not yet exist. Following M2-B02's own already-established, honest resolution for exactly this situation: (1) this blueprint's own hand-authored/structurally-asserted test fixtures live under `crates/chunk-storage/tests/` — already a protected path (TEST-D46 pattern `crates/*/tests/**`), so they cannot be silently altered by a future implementation changeset regardless of the manifest's absence; (2) one `#[ignore]`d oracle-compatibility test (Acceptance tests, below) is committed pending `rc-test-harness` (TEST-D7), exactly mirroring `rc-nbt`'s own `oracle_compatibility.rs` test — asserting `DataVersion == 4903` and `Status == "minecraft:full"` decode correctly from a real, freshly-produced vanilla chunk once that harness exists; (3) a future infrastructure blueprint that stands up `rc-golden-data` should register this blueprint's fixtures in its manifest at that time — not claimed as already done here.

## Deliverables

### `crates/chunk-storage/Cargo.toml` (modify — add three normal dependencies, all already workspace-pinned)

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
serde = { workspace = true }
postcard = { workspace = true }
thiserror = { workspace = true }
io-uring = { workspace = true, optional = true }

[dev-dependencies]
proptest = { workspace = true }

[features]
io_uring = ["dep:io-uring"]
```

(`rc-core`/`rc-nbt`/`rc-registries`/`bevy_ecs`/`io-uring`/`proptest` lines are M2-B01's existing lines, reproduced unchanged for a complete file. `serde`, `postcard`, `thiserror` are this blueprint's own three additions — all already present in the root `[workspace.dependencies]` table per `12-workspace-structure.md` (`serde` since M0-B02/TEST-D27-adjacent tooling, `postcard = "1.1.3"` per CLUSTER-D12, `thiserror` used project-wide) — no new version is pinned anywhere by this blueprint.)

### `crates/chunk-storage/src/lib.rs` (modify — add two module declarations and their re-exports to M2-B01's existing list)

```rust
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
mod chunk_nbt;
mod snapshot;

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
pub use chunk_nbt::{
    BiomeNames, BlockStateNames, ChunkNbtCodec, ChunkNbtDocument, ChunkNbtError,
    DATA_VERSION, MIN_SECTION_Y,
};
pub use snapshot::{ChunkSnapshot, SnapshotError, SnapshotLightSection, RC_CHUNK_SNAPSHOT_VERSION};
```

(Every line from `mod bits;` through `pub use chunk_key::ChunkKeyTag;` is M2-B01's existing, unmodified content, reproduced for a complete file.)

### `crates/chunk-storage/src/chunk_nbt.rs` (new)

```rust
use crate::{
    BiomeColumn, BiomeId, BlockEntityIndex, BlockStateColumn, BlockStateId, ChunkKeyTag,
    ChunkPersistenceState, ChunkStatus, HeightmapSet, LightColumn, PaletteThresholds,
};
use rc_core::{ChunkKey, DimensionId};
use rc_nbt::{borrow, owned, Mutf8Str, Mutf8String};

/// The pinned target's DataVersion (WORLD-D16). Every document this crate writes
/// stamps this value; a loaded document whose `DataVersion` differs is refused.
pub const DATA_VERSION: i32 = 4903;

/// The vanilla `yPos` value every document this crate writes or accepts must carry —
/// `WORLD_MIN_Y / 16` (Context).
pub const MIN_SECTION_Y: i32 = crate::WORLD_MIN_Y / 16;

/// Caller-supplied bridge from this crate's registry-agnostic `BlockStateId` to the
/// vanilla `{Name, Properties}` palette-entry shape (Context's Resolved discrepancy).
/// No implementation ships in this crate.
pub trait BlockStateNames {
    fn name_and_properties(&self, id: BlockStateId) -> Option<(Mutf8String, Vec<(Mutf8String, Mutf8String)>)>;
    fn resolve(&self, name: &Mutf8Str, properties: &[(&Mutf8Str, &Mutf8Str)]) -> Option<BlockStateId>;
}

/// As `BlockStateNames`, for biomes — plain-string palette entries, no properties.
pub trait BiomeNames {
    fn name(&self, id: BiomeId) -> Option<Mutf8String>;
    fn resolve(&self, name: &Mutf8Str) -> Option<BiomeId>;
}

#[derive(Debug, thiserror::Error)]
pub enum ChunkNbtError {
    #[error("unsupported DataVersion: expected {expected}, found {found}")]
    UnsupportedDataVersion { expected: i32, found: i32 },
    #[error("missing required field `{0}`")]
    MissingField(&'static str),
    #[error("field `{0}` has the wrong NBT tag type")]
    WrongFieldType(&'static str),
    #[error("yPos {found} does not match this engine's fixed world bounds (expected {expected})")]
    UnexpectedYPos { expected: i32, found: i32 },
    #[error("section Y {0} is out of the supported light/block range")]
    SectionYOutOfRange(i32),
    #[error("missing required block section for Y {0}")]
    MissingSection(i32),
    #[error("malformed palette in field `{0}`: {1}")]
    MalformedPalette(&'static str, String),
    #[error("block_entities must be empty at M2 scope (no BlockEntityCodec exists yet, WORLD-D6) — found {0} entries")]
    UnsupportedBlockEntities(usize),
    #[error("unknown block state name `{0}` — the supplied BlockStateNames resolver has no match")]
    UnknownBlockStateName(String),
    #[error("unknown biome name `{0}` — the supplied BiomeNames resolver has no match")]
    UnknownBiomeName(String),
    #[error(transparent)]
    Nbt(#[from] rc_nbt::NbtError),
}

/// Every component `chunk_from_nbt` reconstructs, plus the two fields this crate does
/// not store anywhere else (Context: `isLightOn` is a plain passthrough; `extra` is the
/// opaque unknown-tag bag).
pub struct ChunkNbtDocument {
    pub chunk_key: ChunkKeyTag,
    pub blocks: BlockStateColumn,
    pub biomes: BiomeColumn,
    pub light: LightColumn,
    pub heightmaps: HeightmapSet,
    pub block_entities: BlockEntityIndex,
    pub status: ChunkStatus,
    pub persistence: ChunkPersistenceState,
    pub is_light_on: bool,
    pub extra: Vec<(Mutf8String, owned::NbtTag)>,
}

/// Bundles the two registry resolvers and the two `PaletteThresholds` a caller must
/// supply (Context — this crate never bakes in a registry's own size). One `to_nbt`/
/// `from_nbt` call pair per chunk; cheap to construct, holds only borrows and `Copy`
/// values.
pub struct ChunkNbtCodec<'a, N: BlockStateNames, B: BiomeNames> {
    pub block_names: &'a N,
    pub biome_names: &'a B,
    pub block_thresholds: PaletteThresholds,
    pub biome_thresholds: PaletteThresholds,
}

impl<'a, N: BlockStateNames, B: BiomeNames> ChunkNbtCodec<'a, N, B> {
    /// Builds the full vanilla chunk NBT compound (Context: schema, ordering, and the
    /// fixed-default/opaque-extra policy). `extra` is re-emitted verbatim, appended
    /// after every known and fixed-default field, in its given order — pass `&[]` for
    /// a chunk with no captured unknown tags (e.g. one this engine created itself).
    /// Errors only on a non-empty `block_entities` or an `id` the resolvers cannot
    /// name.
    pub fn to_nbt(
        &self,
        chunk_key: ChunkKey,
        blocks: &BlockStateColumn,
        biomes: &BiomeColumn,
        light: &LightColumn,
        heightmaps: &HeightmapSet,
        block_entities: &BlockEntityIndex,
        status: ChunkStatus,
        persistence: ChunkPersistenceState,
        is_light_on: bool,
        extra: &[(Mutf8String, owned::NbtTag)],
    ) -> Result<owned::NbtCompound, ChunkNbtError>;

    /// The inverse. `dimension` is supplied by the caller (the region file the
    /// document was read from names it — vanilla chunk NBT itself carries no
    /// dimension field, only `xPos`/`zPos`) and combined with the loaded `xPos`/`zPos`
    /// into the returned `ChunkKeyTag`.
    pub fn from_nbt(
        &self,
        tag: &borrow::NbtCompound<'_, '_>,
        dimension: DimensionId,
    ) -> Result<ChunkNbtDocument, ChunkNbtError>;
}
```

### `crates/chunk-storage/src/snapshot.rs` (new)

```rust
/// This engine's own internal compatibility counter for `ChunkSnapshot`'s wire shape —
/// independent of Mojang's `DataVersion` (WORLD-D20).
pub const RC_CHUNK_SNAPSHOT_VERSION: u16 = 1;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Default)]
pub struct SnapshotLightSection {
    /// Always exactly 2048 bytes when `Some` — mirrors `LightSection`'s own nibble-
    /// packed array shape (M2-B01), stored as a `Vec` only because `serde`'s derive
    /// does not implement `[u8; 2048]` directly without an extra crate this blueprint
    /// does not add.
    pub sky: Option<Vec<u8>>,
    pub block: Option<Vec<u8>>,
}

/// A flat, self-contained hand-off snapshot of one chunk column (WORLD-D20) — built
/// only from raw scalar data reachable through M2-B01's public accessors, never a
/// `#[derive(Serialize)]` over M2-B01's own component types directly (Context: those
/// types derive no `serde` impls and this blueprint does not retroactively add any).
/// Block entities are **not** captured (Context — WORLD-D6's codec does not exist
/// yet).
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct ChunkSnapshot {
    pub chunk_key: rc_core::ChunkKey,
    /// Section-major flat array of raw block-state ids, length always
    /// `SECTION_COUNT * SECTION_BLOCKS` (98304); entry `section * 4096 + block_index`
    /// (`crate::column::block_index`'s own within-section convention).
    pub block_ids: Vec<u32>,
    /// Section-major flat array of raw biome ids, length always
    /// `SECTION_COUNT * SECTION_BIOME_CELLS` (1536).
    pub biome_ids: Vec<u32>,
    /// One entry per `LIGHT_SECTION_COUNT` (26) light section, ascending index order.
    pub light_sections: Vec<SnapshotLightSection>,
    /// Six flat 256-entry raw-value arrays (`HeightmapSet::raw`'s own convention),
    /// indexed in `HeightmapKind::ALL`'s declared order.
    pub heightmaps: [Vec<u16>; 6],
    /// `0` = `ChunkGenStatus::Generating`, `1` = `ChunkGenStatus::Full` — the same
    /// mapping `chunk_nbt`'s `Status` field uses.
    pub gen_status: u8,
    pub dirty: bool,
    pub last_saved_tick: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("snapshot bytes truncated before the 2-byte format_version prefix")]
    Truncated,
    #[error("unsupported ChunkSnapshot format_version: expected {expected}, found {found}")]
    UnsupportedVersion { expected: u16, found: u16 },
    #[error("postcard decode error: {0}")]
    Decode(#[from] postcard::Error),
}

/// Encodes `snapshot` as `[format_version: 2 bytes, big-endian][postcard-encoded body]`
/// — the version prefix is raw, fixed-width bytes, never itself postcard-encoded
/// (Context: this is what makes `peek_snapshot_version` decodable without knowing any
/// later version's body shape).
pub fn encode_snapshot(snapshot: &ChunkSnapshot) -> Vec<u8>;

/// Reads only the 2-byte version prefix, without attempting to decode the body.
pub fn peek_snapshot_version(bytes: &[u8]) -> Result<u16, SnapshotError>;

/// Full decode. `SnapshotError::UnsupportedVersion` if the prefix does not equal
/// `RC_CHUNK_SNAPSHOT_VERSION` (Context: exact-match policy, no migration, mirroring
/// WORLD-D16 on this second, independent versioning axis).
pub fn decode_snapshot(bytes: &[u8]) -> Result<ChunkSnapshot, SnapshotError>;
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test changeset is every file listed below, plus `crates/chunk-storage/src/{chunk_nbt.rs,snapshot.rs}` with every function body from the Deliverables signatures replaced with `todo!()` (struct/enum field lists, trait definitions, derives, and doc comments stay exactly as specified), plus the `crates/chunk-storage/src/lib.rs` and `Cargo.toml` edits. The implementation changeset (Implementation steps, below) fills in real bodies only; it must not modify any file under `crates/chunk-storage/tests/`, must not weaken any assertion, and must not change any type's field list, derive list, trait definition, or public function signature from what the test changeset already compiled against.

### `crates/chunk-storage/tests/common/mod.rs` — shared test fixtures (not a standalone test binary — Rust's own `tests/common/mod.rs` convention)

```rust
// Synthetic, hand-authored test-only names — never real Mojang data (Context).
pub struct MockBlockNames;
impl rc_chunk_storage::BlockStateNames for MockBlockNames {
    // id 0 = "test:air" (no properties); id 1 = "test:bedrock" (no properties);
    // id 2 = "test:dirt" (no properties); id 3 = "test:grass_block" (no properties) —
    // mirroring M1-B05's own four-block superflat set, renamed into this crate's own
    // test namespace since it must not depend on rc-protocol (Context).
    // id 4 = "test:door", properties returned in a DELIBERATELY non-alphabetical
    // order — {"open": "false", "facing": "north", "half": "lower"} — to prove
    // to_nbt's own sort-before-write rule (Context's "Property-compound ordering").
    // ids 100..500 (400 values) = "test:distinct_<id>" (no properties) — the palette
    // this blueprint's >256-distinct-values-in-one-section test uses.
    // every other id -> None.
}
impl rc_chunk_storage::BiomeNames for MockBiomeNames { /* 0 = "test:plains", else None */ }

pub fn thresholds() -> (rc_chunk_storage::PaletteThresholds, rc_chunk_storage::PaletteThresholds) {
    // (PaletteThresholds::blocks(15), PaletteThresholds::biomes(4)) — the same worked
    // values M2-B01's own tests already use for "real-shaped" thresholds.
}

/// Builds a fully populated, self-consistent set of the seven M2-B01 data components
/// plus a `ChunkKeyTag`, matching M1-B05's own superflat layer content (bedrock/dirt/
/// grass/air in section 0, air everywhere else, single "test:plains" biome, every
/// light section `LightSection::default()`), status `Full`, `dirty: false`,
/// `last_saved_tick: 0`. Returned as a tuple/struct the round-trip tests destructure
/// and compare field-by-field against a decoded `ChunkNbtDocument`.
pub fn superflat_fixture() -> /* ... */ ();
```

### `crates/chunk-storage/tests/chunk_nbt_schema.rs`

1. `all_air_chunk_has_24_uniform_sections_and_no_data_arrays` — build every one of the 24 `BlockStateColumn`/`BiomeColumn` sections as `SingleValue` (air / plains, never `set`), all light sections `None`/`None`, `is_light_on = false`, encode via `ChunkNbtCodec::to_nbt`; decode the result with `rc_nbt::read_owned`; assert: `DataVersion == 4903`, `xPos`/`zPos` match the input `ChunkKey`, `yPos == -4`, `Status == "minecraft:full"`, `isLightOn` tag absent (since `false`), `sections.len() == 24` (no light-padding entries, since every light section is `None`/`None`), every section has `Y` in `-4..=19`, every section's `block_states.palette.len() == 1` with `Name == "test:air"` and no `Properties` tag, no `block_states.data` tag present, `biomes.palette == ["test:plains"]`, no `biomes.data` tag; `Heightmaps` compound has exactly the 4 keys `WORLD_SURFACE`/`OCEAN_FLOOR`/`MOTION_BLOCKING`/`MOTION_BLOCKING_NO_LEAVES`, each a 37-long `LongArray`, and does **not** contain `WORLD_SURFACE_WG`/`OCEAN_FLOOR_WG`; `block_entities` is an empty list; `LastUpdate == 0`; `InhabitedTime == 0`; `structures`/`block_ticks`/`fluid_ticks`/`PostProcessing` are present with their fixed empty shapes (Context).
2. `superflat_section_zero_matches_a_hand_computed_indirect_palette` — using `common::superflat_fixture()`, encode via `to_nbt`; decode section `Y == -4` (block index `0`); assert `block_states.palette` is exactly `["test:bedrock", "test:dirt", "test:grass_block", "test:air"]` **in first-encountered order** scanning `iter()` from local index `0` upward (bedrock at y=-64 is encountered first) — the same "insertion-order-of-first-occurrence" rule the research doc's own §8 note requires reproducing for byte-identical disk output; assert `block_states.data`'s decoded `LongArray` (`unpack_bits` at `bits_per_entry = 4`, matching the `ceil_log2(4) == 2` value floor-bumped to `max(4, 2) == 4`, Context's on-disk floor) reproduces the exact per-cell local indices the fixture's own layer table implies (`4096` values, computed via this test's own reference `unpack_bits` call, not hand-typed one-by-one); assert `block_states.palette.len() == 4` gives `bits_per_entry == 4` per this blueprint's own on-disk rule (`max(4, ceil_log2(4)) == 4`), independent of whatever bit width the in-memory `PalettedContainer` itself happens to be using.
3. `dimension_and_yPos_round_trip_through_from_nbt` — encode a fixture with `ChunkKey::new(DimensionId::THE_NETHER, 7, -3)`; decode via `ChunkNbtCodec::from_nbt(&tag, DimensionId::THE_NETHER)`; assert `document.chunk_key.0 == ChunkKey::new(DimensionId::THE_NETHER, 7, -3)` — proving `xPos`/`zPos` (not the dimension, which NBT never carries) round-trip and the caller-supplied `dimension` is what actually lands in the reconstructed key.
4. `wrong_dimension_argument_produces_a_different_key_not_an_error` — same encoded bytes as test 3, decoded instead with `DimensionId::OVERWORLD`; assert `from_nbt` still succeeds (`Ok`) but `document.chunk_key.0.dimension == DimensionId::OVERWORLD` — proves the API contract precisely: this crate never validates dimension consistency (it has no way to), the caller owns that guarantee.

### `crates/chunk-storage/tests/chunk_nbt_roundtrip.rs` — the byte-identity guarantee

1. `load_of_save_reproduces_every_component_for_the_superflat_fixture` — `common::superflat_fixture()`, `extra = &[]`; `to_nbt` then `from_nbt`; assert, component by component (via explicit cell-by-cell helper assertions this test file defines locally — none of M2-B01's component types derive `PartialEq`, so a derived `assert_eq!` is not available): every `BlockStateColumn::get(x,y,z)` for all `16*384*16` cells matches; every `BiomeColumn::get` for all `16*24*16*(4/16... )` quart-cells (`4*24*4` per chunk) matches; every `LightColumn::section(i).sky`/`.block` matches (`None`s stay `None`); every `HeightmapSet::world_y(kind,x,z)` for all 6 kinds × 256 columns matches (including the two `_Wg` kinds — Context's exact-reconstruction argument); `block_entities.entities().is_empty()` on both sides; `status == ChunkStatus(ChunkGenStatus::Full)` on both sides; `persistence == ChunkPersistenceState { dirty: false, last_saved_tick: 0 }` (load always sets `dirty: false`, Context).
2. `load_of_save_round_trips_for_an_all_air_uninhabited_chunk` — the all-air fixture from `chunk_nbt_schema.rs` test 1's own construction (factored into `common`, not duplicated); same component-by-component assertions as test 1.
3. `load_of_save_round_trips_with_partial_light_data` (`proptest!`, bounded case count) — starting from the superflat fixture, randomly set `Some`/`None` and, when `Some`, a random `[u8; 2048]` pattern independently on each of the 26 light sections' `sky`/`block` fields (52 independent random choices per generated case); `to_nbt` then `from_nbt`; assert every one of the 26 sections' `sky`/`block` matches exactly, including the two padding sections (index `0`/`25`) whose presence in the `sections` NBT list itself depends on whether at least one of `sky`/`block` is `Some` for that generated case (assert the decoded document's light data is correct regardless of which padding entries happened to be omitted from the wire).
4. `extra_fields_round_trip_when_present_on_reload` — encode the superflat fixture with a non-empty `extra` (one synthetic `("custom_test_tag".into(), owned::NbtTag::Int(42))` entry, standing in for a genuinely-unrecognized future field); `to_nbt` then `from_nbt`; assert `document.extra == vec![("custom_test_tag".into(), owned::NbtTag::Int(42))]` exactly — proves the opaque bag is not silently dropped, and that re-supplying it to a second `to_nbt` call (`to_nbt(..., extra = &document.extra)`) then `from_nbt` again reproduces the same `extra` a second time (idempotent round-trip, not merely single-pass-correct).

### `crates/chunk-storage/tests/chunk_nbt_palette_edge_cases.rs`

1. `property_bearing_block_writes_properties_sorted_alphabetically` — a section containing exactly two distinct values, `test:air` (id 0) and `test:door` (id 4, whose mock resolver deliberately returns properties out of order — `common`'s own documented setup); encode; decode the `test:door` palette entry's `Properties` compound via `NbtCompoundExt`/plain iteration; assert its keys appear in the exact order `["facing", "half", "open"]` (alphabetical), regardless of the resolver's own returned order.
2. `zero_property_block_omits_properties_tag_entirely` — the all-air fixture's own `test:air` palette entry; assert `palette_entry.compound("Properties").is_none()` (not `Some(<empty compound>)`).
3. `single_value_section_omits_data_array_for_both_blocks_and_biomes` — restates `chunk_nbt_schema.rs` test 1's own assertion as an explicit, standalone "palette edge case" — kept as its own named test per this blueprint's own Acceptance-tests enumeration discipline (not a duplicate in spirit: this file groups every palette-shape edge case together for a reader auditing WORLD-D2/D11 coverage in one place).
4. `biome_palette_entries_are_plain_strings_not_compounds` — a section with two distinct biomes (`test:plains`, plus a second synthetic mock biome id this test's own local resolver adds); encode; decode `biomes.palette` via `NbtList`'s string-list accessor (not a compound accessor — asserting the *shape*, since attempting `.compound()` on a plain-string list entry would itself fail/return `None`, which this test asserts as the negative half of the proof).
5. `over_256_distinct_block_states_in_one_section_forces_wider_on_disk_packing_than_the_in_memory_container_uses` — build one `BlockStateColumn` section by `set`-ing 300 distinct ids (`test:distinct_100`..`test:distinct_399`, `common`'s own 400-entry mock range) across 300 of its 4096 cells (remaining cells stay `test:air`, id 0) — 301 total distinct values, which promotes the **in-memory** `PalettedContainer` (`PaletteThresholds::blocks(15)`, `max_indirect_bits == 8`) to wire-form `Palette::Direct { bits_per_entry: 15 }` (verify this precondition via `section.bits_per_entry() == 15` before encoding, as a sanity check the test setup actually exercises the intended in-memory state); encode via `to_nbt`; decode; assert `block_states.palette.len() == 301` and the on-disk `data` array's bit width (recovered from its length: `ceil(4096 / (64 / bits)) `, solved for the smallest `bits` consistent with the array's actual `LongArray` length) equals `ceil_log2(301) == 9` — **not** `15` — proving the encoder re-derived the on-disk width from the actual distinct-value count via `iter()`, never copied the in-memory container's own wire-form `Direct` bit width. Decode every one of the 4096 cells back (via `unpack_bits` + palette lookup, this test's own local helper) and assert it matches the original `set` calls exactly.
6. `same_section_encoded_twice_produces_byte_identical_output` — the test-5 section, encoded via `to_nbt` twice independently (two separate calls, not a clone-and-compare); assert the two resulting `Vec<u8>` (via `write_owned` on each) are byte-for-byte equal — the concrete proof that `to_nbt`'s `HashMap`-based first-seen-order palette derivation (Context) is itself deterministic per call, not incidentally order-dependent on `HashMap` iteration (which `std::collections::HashMap` explicitly does not guarantee stable across processes) — since the algorithm derives insertion order from `iter()`'s already-fixed index order, not from iterating the `HashMap` itself, byte-identical output is guaranteed by construction; this test is the regression guard for that guarantee.

### `crates/chunk-storage/tests/chunk_nbt_error_cases.rs`

Each case asserts the exact `ChunkNbtError` variant (via `matches!`), not just `.is_err()` — a stronger requirement than `rc-nbt`'s own `malformed_input_rejection.rs` precedent, appropriate here since every error case below is a specific, precisely-triggerable schema violation rather than arbitrary-byte fuzzing:

1. `wrong_data_version_is_rejected` — a hand-built minimal compound identical to the all-air fixture's own output except `DataVersion` patched to `4902`; `from_nbt` returns `Err(ChunkNbtError::UnsupportedDataVersion { expected: 4903, found: 4902 })`.
2. `wrong_ypos_is_rejected` — as above, `yPos` patched to `-5`; `Err(ChunkNbtError::UnexpectedYPos { expected: -4, found: -5 })`.
3. `missing_block_section_is_rejected` — the all-air fixture's own `sections` list with the `Y == 3` entry removed before decoding; `Err(ChunkNbtError::MissingSection(3))`.
4. `non_empty_block_entities_on_save_is_rejected` — a `BlockEntityIndex` with one pushed (dummy) `Entity` passed to `to_nbt`; `Err(ChunkNbtError::UnsupportedBlockEntities(1))`.
5. `non_empty_block_entities_on_load_is_rejected` — the all-air fixture's own output with a synthetic one-entry `block_entities` list spliced in before decoding; `Err(ChunkNbtError::UnsupportedBlockEntities(1))`.
6. `out_of_range_local_palette_index_is_rejected` — a hand-built `block_states` compound whose `data` `LongArray` encodes a local index `>= palette.len()` for at least one cell (e.g. a 2-entry palette with a `data` array whose packed values include a `2`); `Err(ChunkNbtError::MalformedPalette("block_states", _))`.
7. `unresolvable_block_state_name_on_save_is_rejected` — a `BlockStateColumn` section containing `BlockStateId(9999)` (outside every range `MockBlockNames` recognizes); `to_nbt` returns `Err(ChunkNbtError::UnknownBlockStateName(_))`.
8. `unresolvable_block_state_name_on_load_is_rejected` — a hand-built palette entry `{Name: "test:does_not_exist"}`; `from_nbt` returns `Err(ChunkNbtError::UnknownBlockStateName(_))` (via `MockBlockNames::resolve` returning `None`).

### `crates/chunk-storage/tests/snapshot_postcard.rs`

1. `chunk_snapshot_round_trips_through_encode_decode` (`proptest!`, bounded) — a hand-written `proptest::strategy::Strategy` generating a `ChunkSnapshot` with `block_ids`/`biome_ids` of the exact fixed lengths (`98304`/`1536`), `light_sections.len() == 26` with random `Option<[u8;2048]>` presence per side, `heightmaps` six `Vec<u16>` of length 256, `gen_status ∈ {0,1}`; `encode_snapshot` then `decode_snapshot`; assert the result equals the original (`ChunkSnapshot` derives `PartialEq`).
2. `peek_snapshot_version_reads_without_decoding_the_body` — `encode_snapshot(&some_snapshot)`; corrupt every byte from index `2` onward (garbage, not valid postcard); `peek_snapshot_version` on the corrupted bytes still returns `Ok(RC_CHUNK_SNAPSHOT_VERSION)` (proves it never attempts to decode the body).
3. `mismatched_version_is_rejected_without_attempting_a_body_decode` — take a real `encode_snapshot` output, overwrite its first 2 bytes to `[0x00, 0x63]` (`99`, an unsupported version) leaving the (real, valid-for-version-1) body bytes otherwise untouched; `decode_snapshot` returns `Err(SnapshotError::UnsupportedVersion { expected: 1, found: 99 })` — **not** a `Decode` error, proving the version check happens strictly before any attempt to interpret the body under the wrong schema.
4. `truncated_prefix_is_rejected` — `peek_snapshot_version(&[0x00])` (1 byte) and `decode_snapshot(&[])` (0 bytes) both return `Err(SnapshotError::Truncated)`.
5. `dimension_and_chunk_coordinates_round_trip` — a `ChunkSnapshot` built with `chunk_key: ChunkKey::new(DimensionId::THE_END, -12, 8)`; encode/decode; assert the decoded `chunk_key` is exactly equal.

### `crates/chunk-storage/tests/chunk_nbt_oracle_compatibility.rs`

```rust
#[ignore = "requires a vanilla-produced chunk NBT sample from rc-test-harness (TEST-D7), not yet implemented — see issue #<TRACKING_ISSUE, opened by the implementer at commit time>"]
#[test]
fn decodes_a_real_vanilla_full_chunk_without_error() {
    // Path convention matches rc-nbt's own oracle_compatibility.rs precedent
    // (M2-B02): oracle/26.2/harness/samples/region/r.0.0.mca, read + decompressed by
    // this test's own minimal inline zlib-unwrap (this blueprint does not depend on
    // rc-anvil, which does not exist yet — M2-B02/a sibling blueprint's scope), then
    // decoded via rc_nbt::read_owned and asserted: DataVersion == 4903, Status ==
    // "minecraft:full", yPos == -4.
}
```

## Implementation steps

1. **`Cargo.toml`.** Add the three dependency lines exactly as Deliverables. Observable: `cargo metadata` resolves; `cargo build -p rc-chunk-storage` fails only on `chunk_nbt.rs`/`snapshot.rs`'s remaining `todo!()`s.
2. **`snapshot.rs` — the simpler of the two, implement first.** `encode_snapshot`: `let mut out = RC_CHUNK_SNAPSHOT_VERSION.to_be_bytes().to_vec(); out.extend(postcard::to_stdvec(snapshot).expect("ChunkSnapshot is always serializable")); out`. `peek_snapshot_version`: bounds-check `bytes.len() >= 2` (else `Truncated`), `u16::from_be_bytes([bytes[0], bytes[1]])`. `decode_snapshot`: call `peek_snapshot_version`; if it does not equal `RC_CHUNK_SNAPSHOT_VERSION`, `Err(UnsupportedVersion{..})`; else `postcard::from_bytes(&bytes[2..]).map_err(SnapshotError::from)`. Observable: `snapshot_postcard.rs`'s 5 cases pass.
3. **`chunk_nbt.rs` — palette re-derivation helper (private, shared by both block and biome encoding).** `fn disk_palette_and_data<T: Copy + Eq + std::hash::Hash>(values: impl Iterator<Item = T>, floor_bits: u32) -> (Vec<T>, Option<Vec<u64>>)`: iterate `values` once, maintaining a `Vec<T>` (`palette`, insertion order) and a `HashMap<T, u32>` (`seen`, value → local index) plus a `Vec<u32>` (`locals`, one entry per input value); for each value, look it up in `seen` — if absent, push to `palette` and insert into `seen` at the new index; always push the resolved local index to `locals`. If `palette.len() == 1`, return `(palette, None)`; else `bits = max(floor_bits, crate::ceil_log2(palette.len() as u32))`, return `(palette, Some(crate::pack_bits(&locals, bits).into_vec()))` (converting `Box<[u64]>` to `Vec<u64>` via `.into_vec()`). This one function is used for both blocks (`floor_bits = 4`) and biomes (`floor_bits = 1`) — the exact same algorithm, only the floor differs (Context).
4. **`chunk_nbt.rs` — `to_nbt`.** Build the root compound field by field, in this fixed order: `DataVersion: Int(DATA_VERSION)`, `xPos: Int(chunk_key.x)`, `zPos: Int(chunk_key.z)`, `yPos: Int(MIN_SECTION_Y)`, `Status: String(<mapped per Context>)`, `LastUpdate: Long(persistence.last_saved_tick as i64)`, `InhabitedTime: Long(0)`, conditionally `isLightOn: Byte(1)` only if `is_light_on` (omitted otherwise — NBT has no native Boolean tag; vanilla and this crate alike encode it as a Byte, `1`/`0`, `simdnbt`'s own `borrow`/`owned` accessor naming for a boolean-shaped Byte tag confirmed against the installed 0.10.0 API surface at implementation time, mirroring `rc-nbt`'s own identically-worded verification note), `sections: List(<built per the light-section-Y-mapping table in Context, step 5 below>)`, `block_entities: List(<empty, error first if non-empty>)`, `Heightmaps: Compound(<4 LongArrays, step 6 below>)`, `block_ticks: List([])`, `fluid_ticks: List([])`, `structures: Compound({starts: Compound({}), References: Compound({})})`, `PostProcessing: List(<24 empty Short lists>)`, then every `(name, tag)` in `extra`, appended verbatim in order. Return the assembled `owned::NbtCompound`. Observable: `chunk_nbt_schema.rs` test 1 passes once steps 5/6 below also land.
5. **`chunk_nbt.rs` — section building (part of `to_nbt`).** For `block_index in 0..24`: `vanilla_y = MIN_SECTION_Y + block_index`; call `disk_palette_and_data` on `blocks.section(block_index).iter()` (floor 4) and on `biomes.section(block_index).iter()` (floor 1); for each block palette entry, call `block_names.name_and_properties(id)` (propagate `UnknownBlockStateName` on `None`), sort its returned properties by name bytes ascending, build `{Name: String(name), [Properties: Compound({sorted key/value String pairs})]}`; for each biome palette entry, call `biome_names.name(id)` (propagate `UnknownBiomeName` on `None`) and push a plain `String` tag. Assemble `block_states: {palette: List(<compounds>), [data: LongArray(<words as i64>)]}` and `biomes: {palette: List(<strings>), [data: LongArray(<words as i64>)]}`. Read `light.section(block_index + 1)` (the light-index offset, Context) for optional `BlockLight`/`SkyLight` `ByteArray` tags (2048 bytes each, present iff `Some`). Push `{Y: Byte(vanilla_y as i8), block_states, biomes, [BlockLight], [SkyLight]}`. Before/after this loop, handle the two padding light indices (`0` and `25`) per the Context table: build `{Y: Byte(<-5 or 20> as i8), [BlockLight], [SkyLight]}` and push it **only if** at least one of `sky`/`block` is `Some`; insert the below-padding entry (if present) before the loop's output, the above-padding entry (if present) after. Observable: `chunk_nbt_schema.rs` test 2, `chunk_nbt_palette_edge_cases.rs`'s 6 cases pass.
6. **`chunk_nbt.rs` — heightmaps (part of `to_nbt`).** For each of the 4 persisted `HeightmapKind`s (`WorldSurface`→`"WORLD_SURFACE"`, `OceanFloor`→`"OCEAN_FLOOR"`, `MotionBlocking`→`"MOTION_BLOCKING"`, `MotionBlockingNoLeaves`→`"MOTION_BLOCKING_NO_LEAVES"`), collect `heightmaps.raw(kind, x, z)` for `x`/`z` in vanilla's own row-major XZ order (`x + z*16`, matching `HeightmapSet`'s own internal convention per M2-B01 Implementation step 6) into a `[u32; 256]`, `crate::pack_bits(&that, 9)`, convert to `i64`s, wrap as `LongArray`. Observable: `chunk_nbt_schema.rs` test 1's `Heightmaps` assertions pass.
7. **`chunk_nbt.rs` — `from_nbt`, root fields.** Require and validate `DataVersion`/`yPos` first (fail fast, before touching anything else) via direct `NbtCompound::int`/`.get().and_then(...)` calls (not `NbtCompoundExt`'s `require_*`, since this blueprint's own `ChunkNbtError` variants carry different, more specific data than `rc_nbt::SchemaError` — this crate defines its own error taxonomy rather than reusing `rc-nbt`'s schema layer wholesale, a deliberate choice restated in Constraints). Read `xPos`/`zPos`, build `ChunkKeyTag(ChunkKey::new(dimension, xPos, zPos))`. Map `Status` per Context's exact rule. Read `LastUpdate` into `persistence.last_saved_tick` (`persistence.dirty = false`, always, on load). Read `isLightOn` (`Byte` tag; absent ⇒ `false`; present ⇒ `!= 0`). Observable: `chunk_nbt_schema.rs` tests 3/4, `chunk_nbt_error_cases.rs` tests 1/2 pass.
8. **`chunk_nbt.rs` — `from_nbt`, sections.** Iterate the `sections` list; for each compound, read `Y` (`Byte`, required); classify per the Context table (`Y == MIN_SECTION_Y - 1` → below-padding; `Y == MIN_SECTION_Y + 24` → above-padding; `MIN_SECTION_Y <= Y < MIN_SECTION_Y + 24` → real block section at `block_index = Y - MIN_SECTION_Y`; anything else → `Err(SectionYOutOfRange(Y as i32))`). For a real block section: require `block_states`/`biomes` compounds present; decode each palette (block: `Name`+optional `Properties`, resolved via `block_names.resolve`, propagating `UnknownBlockStateName`; biome: plain strings, via `biome_names.resolve`); decode `data` (if present) via `crate::unpack_bits`, or (if absent) treat every cell as local index `0`; validate every decoded local index `< palette.len()` (`MalformedPalette` otherwise) and validate `data`'s own word count matches what `bits::pack_bits` would have produced for `palette.len()`'s derived bit width (`MalformedPalette` on mismatch); build the section via `PalettedContainer::new_single(palette[0], entry_count, thresholds)` then one `set(i, palette[local_indices[i]])` call per `i in 0..entry_count` (Context — the only construction path available through M2-B01's already-committed public API). Read optional `BlockLight`/`SkyLight` into `LightColumn`'s section `block_index + 1`. Track which of the 24 required block-section `Y` values were actually seen; after the loop, any missing one is `Err(MissingSection(y))`. For a padding section: read optional `BlockLight`/`SkyLight` into `LightColumn`'s section `0` or `25` as appropriate; no `block_states`/`biomes` expected or read. Observable: `chunk_nbt_roundtrip.rs`'s 4 cases, `chunk_nbt_error_cases.rs` tests 3/6/7/8 pass.
9. **`chunk_nbt.rs` — `from_nbt`, heightmaps, block entities, extra.** Require the 4 persisted `Heightmaps` keys (`LongArray`, exactly 37 longs each); `unpack_bits` at 9 bits/256 entries; `set_raw` each of the 4 "final" kinds column-by-column, then `set_raw` `WorldSurfaceWg`/`OceanFloorWg` to `WorldSurface`/`OceanFloor`'s just-set values (Context's exact-reconstruction rule). Require `block_entities` present as a `List`; if non-empty, `Err(UnsupportedBlockEntities(len))`; else `BlockEntityIndex::new()`. Iterate the root compound's own entries via `.iter()`; for every key not in the fixed known-field set (`DataVersion`,`xPos`,`yPos`,`zPos`,`Status`,`LastUpdate`,`isLightOn`,`sections`,`block_entities`,`Heightmaps`) and not in the fixed-default set (`InhabitedTime`,`structures`,`block_ticks`,`fluid_ticks`,`PostProcessing`), push `(name.to_owned(), tag.to_owned())` onto `extra`, preserving iteration order. Assemble and return `ChunkNbtDocument`. Observable: `chunk_nbt_roundtrip.rs` test 4, `chunk_nbt_error_cases.rs` tests 4/5 pass.
10. **`lib.rs`.** Wire the two new `mod`/`pub use` blocks exactly per Deliverables. Observable: `cargo build -p rc-chunk-storage` succeeds with zero `todo!()` remaining.
11. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0.
12. **Push and confirm CI.** Both `ubuntu-24.04` and `windows-2025` legs green on a clean checkout (TEST-D50).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `crates/chunk-storage/tests/` (including the new `common/mod.rs`) is committed first, alongside `todo!()`-stubbed `chunk_nbt.rs`/`snapshot.rs` (full field/trait/derive/doc-comment content already final) and the `lib.rs`/`Cargo.toml` edits. The implementation changeset (steps 1–12) fills in real bodies only — it must not edit any test file, must not add/remove/rename any test case listed in Acceptance tests, and must not weaken any assertion (in particular, `chunk_nbt_schema.rs`'s exact palette-order/field-presence assertions and `chunk_nbt_palette_edge_cases.rs`'s exact bit-width assertions must survive unchanged).

(b) **No new external dependencies beyond `serde`/`postcard`/`thiserror`, all already workspace-pinned.** Do not add `bincode`, `rmp-serde`, `anyhow`, `hex`, or any crate not already present in `rc-chunk-storage`'s `Cargo.toml` after this blueprint's own edit.

(c) **Do not add a dependency from `rc-chunk-storage` to `rc-protocol`, `rc-scheduler`, or `rc-mechanics`** — M2-B01's own Constraint (d) and `xtask lint-deps` Rule 2 apply unchanged; this blueprint's `BlockStateNames`/`BiomeNames` resolver-trait seam (Context) exists specifically so a real registry never needs to enter this crate's own dependency graph.

(d) **`xtask lint-deps`'s four rules are unaffected by this blueprint.** `postcard`/`serde`/`thiserror` are external, non-workspace-member crates; adding them to `rc-chunk-storage` creates no new internal edge and therefore cannot violate Rule 1 (`SHARED` reachability — unrelated), Rule 2 (`SIM`/`NETRENDER` isolation — `rc-chunk-storage` is in neither set and these three dependencies are in neither `NETRENDER` nor a and path into it), Rule 3 (`rc-messaging` purity — untouched), or Rule 4 (`rc-mod-api` leaf — untouched).

(e) **No Mojang or third-party reimplementation code.** Every byte-layout fact this blueprint restates is sourced from `docs/research/mc-26.2/{03-world-chunks.md,04-persistence-nbt.md}` and `docs/planning/03-world-chunks-persistence.md`'s own WORLD-D2/D5/D6/D8/D11/D16/D20 (themselves produced under the ASSET-D18/D30 research-role process), cross-checked against live fetches of `minecraft.wiki`'s Chunk format page performed while deriving this blueprint (cited inline in Context, each with its fetch date) — no decompiled source, no third-party reimplementation's code, is consulted or copied while writing any file this blueprint creates. The mock block/biome names this blueprint's own tests use (`common/mod.rs`) are synthetic, hand-invented test fixtures, never extracted `--reports` data or real registered Mojang names.

(f) **Scope boundary — do not implement beyond this blueprint's stated Implements list.** This blueprint does not implement: the Anvil `.mca` region-file reader/writer or `ChunkStorageBackend` trait (WORLD-D12/D17 — a sibling M2 blueprint); any real `BlockStateNames`/`BiomeNames` implementation backed by a generated registry (deliberately deferred, Context); real `InhabitedTime`/scheduled-tick/structure persistence (04/05's future components — this blueprint's fixed-default policy is the explicit, bounded stand-in, Context); the light BFS propagator or a real `isLightOn` derivation (WORLD-D7/D9 — a future mechanics blueprint; this blueprint's `is_light_on` is a plain caller-supplied `bool`, nothing more); `BlockEntityCodec` implementations or any non-empty-`BlockEntityIndex` (de)serialization (WORLD-D6 — `05-game-mechanics.md`); the Stage-9 snapshot-scheduling integration or `RC-IoPool` wiring that will eventually call `to_nbt`/`from_nbt`/`encode_snapshot`/`decode_snapshot` (a future scheduler-integration blueprint — this blueprint delivers only the pure encode/decode functions themselves, no async I/O, no `ChunkStorageBackend` call site). Do not add placeholder implementations of any of these as a shortcut.

(g) **No `unsafe` code.** Every function in this blueprint's deliverables is implementable in 100% safe Rust (NBT tree construction, bit-packing reuse, and `postcard`/`serde` derives are all safe-Rust surface).

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

Expected: every command exits 0. `cargo nextest run -p rc-chunk-storage` now additionally runs `chunk_nbt_schema.rs` (4) + `chunk_nbt_roundtrip.rs` (4, one a bounded proptest case) + `chunk_nbt_palette_edge_cases.rs` (6) + `chunk_nbt_error_cases.rs` (8) + `snapshot_postcard.rs` (5, one a bounded proptest case) = 27 new cases, alongside M2-B01's existing 42, plus exactly 1 skipped case (`chunk_nbt_oracle_compatibility.rs`'s `#[ignore]`d test) — never a silent pass, always reported as skipped. CI (`.github/workflows/ci.yml`, M0-B08) green on both `ubuntu-24.04` and `windows-2025` legs is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
