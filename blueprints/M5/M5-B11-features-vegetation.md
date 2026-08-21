# M5-B11 — Features Tier 2: Vegetation (Trees, Plants, Ocean & Cave Vegetation)

| Field | Content |
|---|---|
| ID | M5-B11 |
| Milestone | M5 — World Generation Parity |
| Prerequisites | M5-B07 (features & decoration driver — this blueprint adds terminal `Feature`/trunk/foliage/root-placer/tree-decorator kinds alongside the exact dispatch/placement machinery M5-B07 already shipped; Context §0 restates every piece of that machinery this blueprint's own new code calls, so no implementer ever needs to open `M5-B07-features-decoration.md` itself). Also the 5-blueprint M5-B12 family — M5-B12a (dripstone/geode/sculk), M5-B12b (nether geology/ice), M5-B12c (underground/structural misc), M5-B12d (fossil/template), and, directly, M5-B12e (`M5-B12e-features-selectors-dispatcher.md` — the family's own final blueprint, which claims the non-vegetation half of M5-B07's own deferred `Feature`-kind backlog together with its four siblings, and defines the real, complete combined dispatcher, `place_configured_feature_all`, with its `ctx: &PlacementCtx` parameter; Context §A/§D restate exactly what this blueprint reuses from it and how the two compose without conflict). Transitively also depends on M5-B01 (RNG core — every RNG formula this blueprint uses is M5-B01's, restated at each call site, never re-derived) and M5-B02/M5-B03 (compiled `WorldgenData` types and the `AnyRandom`/`WorldgenRandom` carrier this blueprint's code is generic over, restated where used). |
| Implements | GEN-D19 (features & placement — this blueprint is one of two parallel/sequenced continuations of the same 11-step decoration pipeline M5-B07 began, the other being the 5-blueprint M5-B12 family; every terminal `Feature` kind this blueprint adds runs through M5-B07's own unmodified `FeatureSorter`/placement-modifier/`run_placement_chain` machinery), GEN-D6 (feature-seed call sites — unchanged mechanism; this blueprint's new terminal algorithms are simply more consumers of the one already-established `set_feature_seed`-derived RNG stream), GEN-D8/D10 (interpreter-over-JSON architecture and float-determinism discipline, restated wherever this blueprint's own new arithmetic touches floats — mushroom cap tapering, foliage-placer radius falloff), GEN-D20 (restated as a non-exception: this blueprint's own confidence-flagged algorithm gaps, like M5-B07's and the M5-B12 family's, are never conflated with GEN-D20's one pinned tie-break). |
| Crates touched | `rc-worldgen` (`crates/worldgen/`) only: a new `src/decoration/vegetation/` module tree (mirroring the M5-B12 family's own `underground/` directory convention, deliberately kept out of M5-B07's `features/` directory so no sibling blueprint's new files collide); one additive modification to M5-B07's already-shipped `src/decoration/features/tree.rs` (untouched by the M5-B12 family, confirmed — Context §A); one additive modification to M5-B07's `src/decoration/mod.rs` (one new `pub mod` line, independent of M5-B12a's own identical-shaped addition to the same file); one further, composing modification to `src/decoration/driver.rs`'s single per-feature call site, layered on top of M5-B12e's own already-specified version of that same line (Context §D). No `Cargo.toml` change. |
| Estimated scope | L. |

## Goal & Done definition

Close the **vegetation-classified** half of M5-B07's own named deferred backlog (Context §M/§N.5 of that blueprint): 17 terminal `Feature` kinds (trees, ocean/nether vegetation, mushrooms, vines, patches — Context §B's ledger), the full 8-member `TrunkPlacer` and 10-member `FoliagePlacer` families (M5-B07 shipped 2 of each, this blueprint the other 6+8), the 1-member `RootPlacer` family (mangrove, unimplemented anywhere else), and 6 of the 10 `TreeDecorator` kinds — composed with the M5-B12 family's own already-shipped 35-kind non-vegetation half so that, together, this blueprint and the M5-B12 family reach every one of M5-B07's 57 named-deferred kinds (Context §B accounts for all 57: 17 here, 35 across M5-B12a-e, 5 End-exclusive and out of scope for both). This is the vegetation half of what the 10,000-chunk ≥99.9% hash-parity acceptance criterion (GEN-D27, `11-roadmap-milestones.md`) needs; the non-vegetation half is the M5-B12 family's own, already-drafted, and not re-derived here.

Done when:

- [ ] `cargo build -p rc-worldgen` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-worldgen`.
- [ ] Every exact-value test (the `giant_trunk_placer` hand-trace, the `sea_pickle` count hand-trace) reproduces its stated expected value exactly — no tolerance, since every value in this blueprint's own exact-value math is integer, matching this project's established convention (M5-B07's own ore/height traces). Every other kind's acceptance test is explicitly structural (range/determinism/gate checks), matching this blueprint's own honestly-stated confidence flags — never a golden vector this blueprint cannot honestly claim to have verified against real vanilla output.
- [ ] The composed-dispatcher test (`vegetation_dispatch_composition.rs`) proves `place_configured_feature_vegetation` reaches this blueprint's own 17 kinds directly and falls through to M5-B12e's `place_configured_feature_all` (`ctx` forwarded, not dropped) for every kind it does not itself own, byte-identically to calling `place_configured_feature_all` directly with the same `ctx`.
- [ ] `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps` all exit 0 (no new dependency edges).
- [ ] `cargo test --doc -p rc-worldgen` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### 0. Prerequisite API recap — everything this blueprint's own new code calls, restated

M5-B07 shipped `rc-worldgen`'s `decoration/` module tree in full; the 5-blueprint M5-B12 family (M5-B12a-e, Context §A) additively extended it with its own `decoration/underground/` module and, in M5-B12e, the combined dispatcher. This blueprint adds to both without re-deriving either's own architecture. Every symbol this blueprint's new code touches is restated here, verbatim in shape, so no implementer ever needs to open `M5-B07-features-decoration.md` or any of `M5-B12a-features-dripstone-geode-sculk.md` through `M5-B12e-features-selectors-dispatcher.md` themselves.

**RNG carrier and draw primitives** (`crate::random::WorldgenRandom<crate::noise::AnyRandom>`, M5-B01's own crate re-exported through M5-B03 exactly as M5-B07/M5-B12 consume it): every terminal algorithm and placer in this blueprint receives `random: &mut WorldgenRandom<AnyRandom>` and draws through the same `RcRandomSource` trait:
- `random.next_int_bounded(bound: i32) -> i32` — uniform over `[0, bound)`. `WorldgenRandom` ALWAYS uses the classic rejection-loop algorithm (`bits = next_bits(31); val = bits % bound; loop until (bits - val + bound - 1) >= 0 in wrapping 32-bit arithmetic`) regardless of the wrapped backend (M5-B01's own documented quirk, unchanged).
- `random.next_int_between_inclusive(min: i32, max_inclusive: i32) -> i32` = `next_int_bounded(max_inclusive - min + 1) + min`, ONE draw.
- `random.next_float() -> f32` — uniform `[0.0, 1.0)`, ONE draw (`next_bits(24) as f32 * 2^-24`).
- `random.next_bool() -> bool` — ONE draw (`next_bits(1) != 0`).
- `random.next_double() -> f64` — uniform `[0.0, 1.0)`, ONE draw (two `next_bits` calls internally, counted as one logical draw here exactly as M5-B07 counted `next_float`).

**Providers** (`crate::decoration::providers`, M5-B07 Deliverables, unchanged, reused verbatim):
- `sample_int_provider(p: &IntProvider, random) -> i32` — `Constant(n)`: `n`, ZERO draws. `Uniform{min,max}`: `random.next_int_between_inclusive(min,max)`, ONE draw. `Other(_)`: `panic!` (M5-B07's own tier boundary, unchanged).
- `sample_height_provider(p: &HeightProvider, random) -> i32` — M5-B07's own six-kind table (Constant/Uniform high confidence, BiasedToBottom/VeryBiasedToBottom/Trapezoid/WeightedList low-moderate), unchanged, reused verbatim by every kind in this blueprint that needs a `HeightProvider` field.
- `sample_block_state_provider(p: &BlockStateProvider, random, resolver: &dyn BlockStateResolver) -> rc_chunk_storage::BlockStateId` — `SimpleStateProvider`: ZERO draws. `WeightedStateProvider`: ONE draw, cumulative-weight selection. `Unsupported`: `panic!`. `BlockStateProvider` itself (`crate::decoration::providers::BlockStateProvider`) is M5-B07's own type — every `BlockStateProvider`-typed field in this blueprint's new config structs reuses that exact same compiled type, never a new one.

**Placement-chain re-entry** (`crate::decoration::modifiers::{run_placement_chain, PlacementCtx}`, M5-B07 Deliverables, unchanged):
```rust
pub struct PlacementCtx<'a> {
    pub step: crate::data::DecorationStep,
    pub feature_name: &'a crate::data::ResourceLocation,
    pub biome_defs: &'a std::collections::BTreeMap<crate::data::ResourceLocation, crate::data::BiomeDefinition>,
    pub biome_names: &'a dyn crate::decoration::context::BiomeNameResolver,
    pub resolver: &'a dyn crate::decoration::context::BlockStateResolver,
    pub props: &'a dyn crate::decoration::context::BlockPropertyResolver,
}
pub fn run_placement_chain(
    placement: &[crate::data::PlacementModifier],
    origin: rc_core::BlockPos,
    world: &mut dyn crate::decoration::context::DecorationWorldAccess,
    ctx: &PlacementCtx,
    random: &mut WorldgenRandom<AnyRandom>,
    place_fn: &mut dyn FnMut(&mut dyn crate::decoration::context::DecorationWorldAccess, rc_core::BlockPos, &mut WorldgenRandom<AnyRandom>),
);
```
This blueprint's own `vegetation_patch`/`waterlogged_vegetation_patch` (Context §J, the only kind here that recurses) invokes `run_placement_chain` re-entrantly exactly as M5-B07's own `random_patch` does (M5-B07 Context §N.6: "recursively invoke `data.placed_features.get(&config.feature)`'s own full `run_placement_chain` at candidate as ITS new origin — a genuine nested full-placement-modifier-chain call") and exactly as M5-B12e's own selector/`sequence` kinds do (M5-B12e Context §K's `delegate` helper). **This blueprint restates one small clarification the same way M5-B12e independently does**: the re-entrant call constructs a **new** `PlacementCtx` identical to the caller's own except `feature_name`, which is rebound to the nested feature's own `ResourceLocation` — this is what makes a `Biome{}` modifier (M5-B07 Context §G.6) buried inside the nested chain check the CORRECT feature's presence in the current biome's step list, not the outer feature's.

**Resolver seams** (`crate::decoration::context`, M5-B07 Deliverables, unchanged):
```rust
pub trait DecorationWorldAccess {
    fn get_block(&self, pos: rc_core::BlockPos) -> rc_chunk_storage::BlockStateId;
    fn set_block(&mut self, pos: rc_core::BlockPos, state: rc_chunk_storage::BlockStateId) -> bool;
    fn biome_at(&self, pos: rc_core::BlockPos) -> rc_chunk_storage::BiomeId;
    fn heightmap_y(&self, kind: rc_chunk_storage::HeightmapKind, x: i32, z: i32) -> i32;
}
pub trait BlockStateResolver {
    fn resolve(&self, spec: &crate::data::BlockStateSpec) -> rc_chunk_storage::BlockStateId;
    fn air(&self) -> rc_chunk_storage::BlockStateId;
}
pub trait BlockPropertyResolver {
    fn is_air_or_replaceable(&self, state: rc_chunk_storage::BlockStateId) -> bool;
    fn is_solid(&self, state: rc_chunk_storage::BlockStateId) -> bool;
    fn is_still_water(&self, state: rc_chunk_storage::BlockStateId) -> bool;
    fn has_sturdy_face(&self, state: rc_chunk_storage::BlockStateId, direction: crate::decoration::context::Direction) -> bool;
    fn would_survive(&self, placing: rc_chunk_storage::BlockStateId, at: rc_core::BlockPos, world: &dyn DecorationWorldAccess) -> bool;
    fn matches_tag(&self, state: rc_chunk_storage::BlockStateId, tag: &str) -> bool;
}
```
`rc_core::BlockPos { x: i32, y: i32, z: i32 }` (M0/M2's own type, unchanged). Every kind in this blueprint reads/writes blocks exclusively through these traits — never through `01`'s tick-time `UpdateContext`, exactly as M5-B07's own Constraints (d) binds (restated as binding on this blueprint too, Constraints below).

**Compiled data types** (`crate::data`, M5-B02's own compiled types, unchanged): `ResourceLocation { namespace: String, path: String }` (parses `"namespace:path"`, defaults `"minecraft"`); `BlockStateSpec { block: ResourceLocation, properties: BTreeMap<String,String> }`; `ConfiguredFeature { feature_type: ResourceLocation, config: serde_json::Value }`; `PlacedFeature { feature: ConfiguredFeatureId, placement: Vec<PlacementModifier> }`; `WorldgenData { configured_features: BTreeMap<ConfiguredFeatureId, ConfiguredFeature>, placed_features: BTreeMap<ResourceLocation, PlacedFeature>, .. }` (every other field unused by this blueprint). `IntProvider`/`HeightProvider` are `crate::data`'s own compiled types, identical shape to the table restated above.

**M5-B07's own tree pipeline entry point being modified** (`crate::decoration::features::tree`, M5-B07 Deliverables — confirmed untouched by the M5-B12 family, Context §A — this blueprint's own Context §E/§F/§G/§H below define the additive extension in full):
```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct TreeConfiguration {
    pub trunk_provider: crate::decoration::providers::BlockStateProvider,
    pub trunk_placer: TrunkPlacerJson,
    pub foliage_provider: crate::decoration::providers::BlockStateProvider,
    pub foliage_placer: FoliagePlacerJson,
    #[serde(default)]
    pub force_dirt: bool,
}
```
M5-B07 already implemented `TrunkPlacerJson::StraightTrunkPlacer`/`BendingTrunkPlacer` and `FoliagePlacerJson::BlobFoliagePlacer`/`SpruceFoliagePlacer` with a shared height formula (`height = base_height + random.next_int_bounded(height_rand_a + 1) + random.next_int_bounded(height_rand_b + 1)`, TWO draws, `height_rand_a` before `height_rand_b`, HIGH confidence) and `BlobFoliagePlacer`'s own algorithm (samples `radius`/`offset` via `sample_int_provider`, TWO draws; per-layer disk with probabilistic corner-skip, ONE `next_int_bounded(2)` draw per exact-corner candidate). This blueprint's own `place_blob_foliage`/`place_spruce_foliage` helpers are assumed to already exist with those exact behaviors and are called directly by this blueprint's own `BushFoliagePlacer`/`JungleFoliagePlacer` (Context §H).

**M5-B07's original `place_configured_feature` and M5-B12e's combined `place_configured_feature_all`** (both restated, neither modified by this blueprint — Context §A/§D explain the composition):
```rust
// M5-B07, `crate::decoration::features::place_configured_feature` — untouched, still dispatches
// only its own 7 original kinds (ore/disk/spring_feature/lake/tree/random_patch/simple_block).
pub fn place_configured_feature(
    feature: &crate::data::ConfiguredFeature, origin: rc_core::BlockPos,
    world: &mut dyn crate::decoration::context::DecorationWorldAccess,
    resolver: &dyn crate::decoration::context::BlockStateResolver,
    props: &dyn crate::decoration::context::BlockPropertyResolver,
    random: &mut WorldgenRandom<AnyRandom>, data: &crate::data::WorldgenData,
);

// M5-B12e, `crate::decoration::underground::place_configured_feature_all` — supersedes the above as
// `decorate_chunk`'s own real call site (M5-B12e Context §S); dispatches M5-B07's 7 kinds (delegated
// verbatim) plus the M5-B12 family's own 35 kinds, else a documented no-op. `ctx: &PlacementCtx` is
// REQUIRED — M5-B12e's own selector/`sequence` kinds (Context §K there) need it to re-enter
// `run_placement_chain`; this blueprint's own fallthrough (Context §D below) forwards its own
// already-available `ctx` rather than dropping it. `UndergroundFeatureContext` is the M5-B12 family's
// own opaque type — this blueprint never constructs or inspects it, only forwards it.
pub fn place_configured_feature_all(
    feature: &crate::data::ConfiguredFeature, origin: rc_core::BlockPos,
    world: &mut dyn crate::decoration::context::DecorationWorldAccess,
    resolver: &dyn crate::decoration::context::BlockStateResolver,
    props: &dyn crate::decoration::context::BlockPropertyResolver,
    random: &mut WorldgenRandom<AnyRandom>, data: &crate::data::WorldgenData,
    ctx: &crate::decoration::modifiers::PlacementCtx,
    bridge: Option<&crate::decoration::underground::UndergroundFeatureContext>,
);
```

### A. Scope boundary — reconciled against the M5-B12 family

M5-B07 named 57 deferred `Feature` kinds (Context §B explains the "57 vs. 56" bookkeeping discrepancy in M5-B07's own summary prose) and 14 deferred trunk/foliage placer kinds, split along a vegetation/non-vegetation axis between this blueprint and the 5-blueprint M5-B12 family (M5-B12a — dripstone/geode/sculk, M5-B12b — nether geology/ice, M5-B12c — underground/structural misc, M5-B12d — fossil/template, M5-B12e — selectors/combinators and the family's own final combined dispatcher, `place_configured_feature_all`, Context §0). `blueprints/M5/M5-B00-index.md` owns the complete, authoritative 64-kind coverage table (7 by M5-B07 + 17 here + 35 across M5-B12a-e + 5 End-exclusive) and the ID-reservation history; this blueprint does not restate that audit, only consumes its result.

**This blueprint reconciles with the M5-B12 family's own real content.** Cross-checking the family's own vegetation-kind list (18 names: `fallen_tree`, `chorus_plant`, `huge_red_mushroom`, `huge_brown_mushroom`, `vines`, `vegetation_patch`, `waterlogged_vegetation_patch`, `seagrass`, `kelp`, `coral_tree`, `coral_mushroom`, `coral_claw`, `sea_pickle`, `bamboo`, `huge_fungus`, `nether_forest_vegetation`, `weeping_vines`, `twisting_vines`) against this blueprint's own independently-derived classification (Context §B) finds **agreement on 17 of those 18 names**; the one discrepancy is `chorus_plant`, listed elsewhere as an M5-B11-owned vegetation kind but genuinely End-dimension-exclusive in real vanilla (the Chorus Plant grows only in the End) — this blueprint does not implement it, for the identical dimension-scope reason M5-B07 §A already puts the whole End out of scope, restated in Context §I. The net effect: `chorus_plant` is unimplemented anywhere in this project — a genuine, small, honestly-flagged gap (Constraints (g)), not a silent omission, and harmless in practice since GEN-D1's own scope already excludes the End dimension entirely, so no real corpus chunk exercises it. Every kind the M5-B12 family implements as non-vegetation (its own 35 kinds — `no_op`, `random_selector`/`weighted_random_selector`/`simple_random_selector`/`random_boolean_selector`/`sequence`, `root_system`, `multiface_growth`, `block_pile`, `sculk_patch` among them) is left untouched by this blueprint, which defers entirely to the family's own already-shipped, self-consistent implementations rather than duplicating them.

**Composition, not modification.** The M5-B12 family explicitly declined to modify M5-B07's own `features/mod.rs`/`place_configured_feature` (M5-B12e's own Constraints (d): that file and function are "frozen"), instead defining its own new, combined dispatcher (`place_configured_feature_all`, Context §0) and having `driver.rs`'s single per-feature call site invoke that instead. This blueprint follows the identical discipline: it does not modify M5-B07's `features/mod.rs` either, and it does not modify the M5-B12 family's own `underground/mod.rs` (outside this blueprint's assigned path in any case). Instead, this blueprint defines its own dispatcher, `place_configured_feature_vegetation` (Context §D), which tries this blueprint's own 17 kinds first and falls through to M5-B12e's `place_configured_feature_all` for everything else — a pure composition, reaching every sibling blueprint's own kinds through one call chain, with `driver.rs`'s call site updated one further time to invoke this blueprint's own outer function (Context §D's own explicit ordering note).

**Dimension tiering** is unchanged from M5-B07's own Context §A: overworld and nether in scope, the End out of scope (Context §I).

### B. The complete 57-kind deferred-feature ledger, reconciled with the M5-B12 family

M5-B07 Context §M's own summary sentence says "56 remaining kinds" but its own enumerated list contains **57** distinct names (mechanically recounted from that blueprint's own text — the M5-B12 family's own derivation independently found the identical discrepancy and used the same resolution: the literal enumerated list, not the summary count, is authoritative). This blueprint additionally notes, as both M5-B07 and the M5-B12 family implicitly rely on: `random_patch` (M5-B07's own 7th implemented kind) does not appear in `docs/research/mc-26.2/05-worldgen.md` §3.13's own 63-name enumeration despite being a real, unambiguous, extremely well-known vanilla feature kind — an omission in that research document's own list, not an error upstream; the true universe of confirmed feature kinds is 64.

**Ownership, reconciled (Context §A):**

| Owner | Count | Kinds |
|---|---|---|
| M5-B07 (already implemented) | 7 | `ore`, `disk`, `spring_feature`, `lake`, `tree`, `random_patch`, `simple_block` |
| **This blueprint (M5-B11) — 17** | 17 | `fallen_tree`, `huge_red_mushroom`, `huge_brown_mushroom`, `vines`, `vegetation_patch`, `waterlogged_vegetation_patch`, `seagrass`, `kelp`, `coral_tree`, `coral_mushroom`, `coral_claw`, `sea_pickle`, `bamboo`, `huge_fungus`, `nether_forest_vegetation`, `weeping_vines`, `twisting_vines` |
| M5-B12a-e (already implemented, not touched here) | 35 | M5-B12a: `large_dripstone`, `speleothem`, `speleothem_cluster`, `geode`, `sculk_patch`. M5-B12b: `delta_feature`, `basalt_columns`, `basalt_pillar`, `netherrack_replace_blobs`, `glowstone_blob`, `iceberg`, `blue_ice`, `freeze_top_layer`, `spike`. M5-B12c: `root_system`, `multiface_growth`, `underwater_magma`, `monster_room`, `block_pile`, `block_column`, `replace_single_block`, `block_blob`, `desert_well`, `void_start_platform`, `fill_layer`, `bonus_chest`. M5-B12d: `fossil`, `template`. M5-B12e: `no_op`, `random_selector`, `weighted_random_selector`, `simple_random_selector`, `random_boolean_selector`, `sequence`, `scattered_ore`. |
| Out of scope (End-exclusive) | 5 | `chorus_plant` (Context §A's own flagged gap), `end_platform`, `end_spike`, `end_island`, `end_gateway` |

`7 + 17 + 35 + 5 = 64` (the true total, `random_patch` included alongside the research corpus's own 63). `17 + 35 + 5 = 57`, matching M5-B07's own literal deferred-list count exactly — every deferred kind is accounted for exactly once between this blueprint, the M5-B12 family, and the out-of-scope set.

**Trunk/foliage/root placer and tree-decorator ledger — entirely this blueprint's own, no split with the M5-B12 family exists for this family** (the M5-B12 family's own scope is strictly the `Feature` registry; `TrunkPlacer`/`FoliagePlacer`/`RootPlacer`/`TreeDecorator` are four separate registries it never touches): 6 trunk placers (`forking_trunk_placer`, `giant_trunk_placer`, `mega_jungle_trunk_placer`, `dark_oak_trunk_placer`, `fancy_trunk_placer`, `cherry_trunk_placer` — M5-B07 already shipped `straight_trunk_placer`/`bending_trunk_placer`), 8 foliage placers (`pine_foliage_placer`, `acacia_foliage_placer`, `bush_foliage_placer`, `fancy_foliage_placer`, `jungle_foliage_placer`, `mega_pine_foliage_placer`, `dark_oak_foliage_placer`, `cherry_foliage_placer` — M5-B07 already shipped `blob_foliage_placer`/`spruce_foliage_placer`), the 1-member `RootPlacer` family (`mangrove_root_placer`, Context §E), and 6 of the 10 `TreeDecorator` kinds (Context §F: `beehive`, `trunk_vine`, `leave_vine`, `cocoa`, `attached_to_leaves`, `attached_to_logs`; the remaining 4 — `pale_moss`, `creaking_heart`, `alter_ground`, `place_on_ground` — are named-deferred).

### C. Confidence-tiering policy, restated

Every algorithm below follows M5-B07's and the M5-B12 family's own already-established discipline exactly: a **high**-confidence restatement is one this blueprint's own derivation pass could pin to an exact, unambiguous public formula (the shared tree-height formula, most RNG draw *counts* and *orders*, every deterministic geometric shape that does not depend on an unverified vanilla constant). A **moderate** or **low-moderate** restatement is this blueprint's own best-effort, internally-consistent reconstruction of a shape the research corpus describes narratively or not at all — flagged explicitly, backed by a *structural* acceptance test (bounds, determinism, gating conditions), never a golden vector this blueprint cannot honestly claim. Every constant this blueprint introduces without a confirmed public source (mushroom cap radii, vine spread probabilities, patch try-counts, and similar) is this blueprint's own named, documented, internally-consistent choice — never presented as vanilla-verified. A future GEN-D27 differential run against the reference vanilla server is the actual reconciliation step for every such flag, matching this project's own established convention (M5-B07 Context §D/§N.4/§N.5, the M5-B12 family's own per-blueprint confidence tables, M5-B04/M5-B06's own identically-flagged items).

### D. The composed dispatcher — `place_configured_feature_vegetation`

```rust
/// Context §D. Tries this blueprint's own 17 kinds (Context §B's ledger); for every
/// other `feature_type`, falls through to M5-B12e's own `place_configured_feature_all`
/// (Context §0), which itself falls through to M5-B07's original 7 kinds, then to a
/// final documented no-op — so this one function is the real, complete entry point
/// covering every sibling blueprint's own work, and is what `driver.rs`'s call site
/// invokes (below). `data`/`ctx` are needed by this blueprint's own
/// `vegetation_patch`/`waterlogged_vegetation_patch`'s re-entrant recursion (Context §J)
/// AND — this is the load-bearing part — `ctx` is forwarded, UNCHANGED, to every
/// fallthrough call to M5-B12e's `place_configured_feature_all`, which requires it
/// (Context §0's own restated signature: 5 of M5-B12e's own kinds re-enter
/// `run_placement_chain` and cannot function without it). `bridge` is likewise forwarded
/// to the M5-B12e fallback, never inspected here.
#[allow(clippy::too_many_arguments)]
pub fn place_configured_feature_vegetation(
    feature: &crate::data::ConfiguredFeature,
    origin: rc_core::BlockPos,
    world: &mut dyn crate::decoration::context::DecorationWorldAccess,
    resolver: &dyn crate::decoration::context::BlockStateResolver,
    props: &dyn crate::decoration::context::BlockPropertyResolver,
    random: &mut WorldgenRandom<AnyRandom>,
    data: &crate::data::WorldgenData,
    ctx: &crate::decoration::modifiers::PlacementCtx,
    bridge: Option<&crate::decoration::underground::UndergroundFeatureContext>,
);
```

**Ordering note, stated once, binding (restated in Constraints too):** this blueprint's own text for `driver.rs`'s per-feature call site (Deliverables) is written as the FINAL, composed form of that one line, superseding M5-B12e's own already-specified version of the identical call site (M5-B12e Context §S). Applying M5-B07's, then the M5-B12 family's (through M5-B12e), then this blueprint's own Deliverables, in that order, converges on one consistent `driver.rs`; M5-B12e's own dispatcher (`place_configured_feature_all`) remains fully intact and reachable either way, just invoked one layer further out, with `ctx` now flowing through it correctly at every layer. This is a real, narrow integration point between independently-derived sibling blueprints, named explicitly rather than silently overwritten (Constraints (g)).

### E. `RootPlacer` — mangrove roots, and the `TreeConfiguration` extension it requires

**`TreeConfiguration` gains one new field** (additive — every existing M5-B07 field is unchanged):
```rust
pub struct TreeConfiguration {
    pub trunk_provider: crate::decoration::providers::BlockStateProvider,
    pub trunk_placer: TrunkPlacerJson,
    pub foliage_provider: crate::decoration::providers::BlockStateProvider,
    pub foliage_placer: FoliagePlacerJson,
    #[serde(default)]
    pub force_dirt: bool,
    /// NEW (this blueprint). `None` for every tree M5-B07 already handles (straight/
    /// bending placers never carry a root system). `Some` only for mangrove-style trees.
    #[serde(default)]
    pub root_placer: Option<RootPlacerJson>,
    /// NEW (this blueprint) — Context §F.
    #[serde(default)]
    pub decorators: Vec<TreeDecoratorJson>,
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RootPlacerJson {
    MangroveRootPlacer {
        root_provider: crate::decoration::providers::BlockStateProvider,
        trunk_offset_y: crate::data::IntProvider,
        max_root_width: i32,
        max_root_length: i32,
        #[serde(default)]
        random_skew_chance: f32,
    },
    #[serde(other)]
    Unsupported,
}
```

**Note, so this is never confused with M5-B12c's own, differently-named `root_system` Feature kind (Context §B's ledger):** `RootPlacer`/`mangrove_root_placer` is a *component of `TreeConfiguration`* (like `TrunkPlacer`/`FoliagePlacer`), consulted only when the `tree` Feature kind places a mangrove-style tree. M5-B12c's `root_system` is a completely different, *standalone* Feature kind (used for e.g. flowering azalea trees with a visible dangling root network) that happens to share a similar-sounding name — the two do not interact and are not the same mechanism.

**Algorithm** (low-moderate confidence — this blueprint's own best-effort reconstruction of a mangrove tree's below-ground root network; the research corpus does not describe this family's byte-level shape). Invoked from `tree::place` (Implementation steps) BEFORE the trunk placer runs, only when `config.root_placer.is_some()`:

1. Gate check, zero RNG: the block directly below `origin` must satisfy `props.matches_tag(state, "minecraft:mangrove_roots_can_grow_through")` (this blueprint's own tag-name choice, standing in for vanilla's real mud/muddy-mangrove-roots gate) — if not, the whole tree placement is a no-op (mirrors vanilla's own "wrong soil, nothing grows" behavior; this blueprint's engine has no separate failure-signaling protocol for terminal features, restated explicitly here rather than silently, matching M5-B07's/M5-B12's own identical convention for their own analogous gates).
2. `trunk_offset = sample_int_provider(&config.trunk_offset_y, random)` (1+ draws, provider-dependent) — the real trunk placer's own `origin` is shifted UP by `trunk_offset` blocks from this root-placer's own `origin` (roots grow down from the tree's nominal base to the mud below; the visible trunk starts above that).
3. For `strand in 0..max_root_width` (ascending index order): draw a horizontal start jitter, `jx = random.next_int_bounded(3) - 1`, `jz = random.next_int_bounded(3) - 1` (TWO draws, X then Z); walk downward from `(origin.x + jx, origin.y, origin.z + jz)` for up to `max_root_length` steps — at each step, draw `skew_roll = random.next_float()` (ONE draw); if `skew_roll < random_skew_chance`, additionally draw `skew_dir = random.next_int_bounded(4)` (ONE draw, mapped via Context §F's fixed N/E/S/W table) and shift the walk one block horizontally in that direction before continuing down; place `sample_block_state_provider(&config.root_provider, random, resolver)` at each visited position; stop the strand early the first time a visited position is NOT air/water/replaceable (reached solid ground). TOTAL draws per strand: `2 + (1 + [1 if skew triggers]) * steps_walked` — genuinely data/RNG-dependent, restated as a draw *order* (jitter, then per-step roll-then-conditional-direction) rather than a fixed count.

The tree driver returns the trunk's own `FoliageAttachment`s (Context §G) unaffected by root placement — roots never contribute foliage attachment points.

### F. `TreeDecorator` pipeline — post-pass, 6 of 10 kinds implemented

**`TreeConfiguration.decorators: Vec<TreeDecoratorJson>`** (Context §E) runs, in declared list order, strictly AFTER the trunk, foliage, and (if present) root placer have all finished writing blocks (`docs/research/mc-26.2/05-worldgen.md` §3.13: "an ordered list of `TreeDecorator`s run after the trunk+foliage are stamped"). Each decorator receives the full set of positions the trunk/foliage placers actually wrote, partitioned by kind:

```rust
/// Accumulated by `tree::place` (Implementation steps) as the trunk/foliage/root
/// placers run, then handed to every decorator in `config.decorators`' own list order.
pub struct TreePlacementLog {
    pub log_positions: Vec<rc_core::BlockPos>,
    pub leaf_positions: Vec<rc_core::BlockPos>,
    pub root_positions: Vec<rc_core::BlockPos>,
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TreeDecoratorJson {
    Beehive { probability: f32 },
    TrunkVine {},
    LeaveVine {},
    Cocoa { probability: f32 },
    AttachedToLeaves { probability: f32, exclusion_radius_xz: i32, exclusion_radius_y: i32, required_empty_blocks: i32, block_provider: crate::decoration::providers::BlockStateProvider, #[serde(default)] directions: Vec<String> },
    AttachedToLogs { probability: f32, exclusion_radius_xz: i32, exclusion_radius_y: i32, required_empty_blocks: i32, block_provider: crate::decoration::providers::BlockStateProvider, #[serde(default)] directions: Vec<String> },
    /// `pale_moss`/`creaking_heart`/`alter_ground`/`place_on_ground` — named-deferred
    /// (this section's own tier boundary below), a documented `debug`-logged no-op,
    /// never a panic, matching M5-B07 Context §M's identical dispatch discipline.
    #[serde(other)]
    Unsupported,
}

/// Dispatches on `dec`'s own variant; `Unsupported` is a documented no-op.
pub fn apply_tree_decorator(
    dec: &TreeDecoratorJson,
    log: &TreePlacementLog,
    world: &mut dyn crate::decoration::context::DecorationWorldAccess,
    resolver: &dyn crate::decoration::context::BlockStateResolver,
    props: &dyn crate::decoration::context::BlockPropertyResolver,
    random: &mut WorldgenRandom<AnyRandom>,
);
```

**Fixed horizontal-direction table** (defined once, here, reused by every decorator in this section, by Context §G/§J's trunk placers and terminal features — never redefined or reordered elsewhere in this blueprint): `dir_offset(dir_idx: i32) -> (i32, i32)` maps `0,1,2,3` to `(dx,dz)` offsets `(0,-1),(1,0),(0,1),(-1,0)` (North, East, South, West respectively); `dir_name(dir_idx: i32) -> &'static str` maps the same indices to the literal strings `"north"`, `"east"`, `"south"`, `"west"`. Both are free functions this blueprint adds to `crate::decoration::vegetation` (Deliverables), so every file below calls the identical mapping rather than each re-deriving its own.

**`beehive`** (`Beehive{probability}`, moderate confidence — task-required exact chance/position mechanism). `roll = random.next_float()` (ONE draw); if `roll >= probability`, stop — zero further draws (the direction draw below is CONDITIONAL on the probability roll succeeding, restated explicitly since it is an easy-to-miss pattern, the same discipline M5-B07 Context §N.1 names for `ore`'s own discard-chance). On success: let `top = log.log_positions.iter().max_by_key(|p| p.y)` (the topmost log, deterministic, zero draws); `dir_idx = random.next_int_bounded(4)` (ONE draw); `candidate = top + 2 * dir_offset(dir_idx)` (two steps out in the chosen direction, `y` unchanged); if `world.get_block(candidate)` is air-or-replaceable AND `log.leaf_positions.contains(&(candidate + (0,1,0)))` (a leaf immediately above — this blueprint's own concrete, testable stand-in for vanilla's real "valid ledge with clearance" check), place `resolver.resolve(&BlockStateSpec{block: "minecraft:bee_nest", properties: {"facing": dir_name(dir_idx)}})`. This blueprint places the bare block only — the bee-nest block-entity's own occupant/honey-level NBT payload is a `05-game-mechanics.md` concern, out of this blueprint's own block-state-only scope, named here as a documented, bounded simplification rather than silently omitted.

**`trunk_vine`** (`TrunkVine{}`, moderate confidence). For `pos` in `log.log_positions` (declared accumulation order), for `dir_idx` in `0..4` (fixed N,E,S,W order): if `world.get_block(pos + dir_offset(dir_idx))` is air, draw `roll = random.next_int_bounded(2)` (ONE draw PER direction PER log, only for air neighbors — matching the same "conditional draw" discipline as `beehive`); place `vine[facing=dir]` iff `roll == 0`.

**`leave_vine`** (`LeaveVine{}`, identical algorithm and draw shape to `trunk_vine`, iterating `log.leaf_positions` instead of `log.log_positions`).

**`cocoa`** (`Cocoa{probability}`, moderate confidence). For `pos` in `log.log_positions`: `roll = random.next_float()` (ONE draw PER log, unconditional); if `roll < probability`: `dir_idx = random.next_int_bounded(4)` (ONE draw, CONDITIONAL on the probability roll succeeding — same discipline as `beehive`); if `world.get_block(pos + dir_offset(dir_idx))` is air, place `cocoa[facing=dir,age=0]`.

**`attached_to_leaves`** / **`attached_to_logs`** (moderate-low confidence, shared shape, differing only in which position list they iterate). For `pos` in the relevant position list: skip (zero draws) if `pos` is within `exclusion_radius_xz`/`exclusion_radius_y` of the tree's own origin (deterministic proximity check); else draw `roll = random.next_float()` (ONE draw); if `roll < probability`: let `dirs = if config.directions.is_empty() { all 4 } else { config.directions }`; draw `dir_idx = random.next_int_bounded(dirs.len() as i32)` (ONE draw, conditional on the probability roll); walk up to `required_empty_blocks` positions in that direction checking each is air (zero draws); if all are, place `sample_block_state_provider(&block_provider, random, resolver)` at the first one.

**Named-deferred (4, documented no-op, this section's own `Unsupported` arm):** `pale_moss`, `creaking_heart`, `alter_ground`, `place_on_ground` — newer/rarer decorators this blueprint's own derivation pass could not confidently restate; owner: a future tree-decoration follow-up, not yet ID-reserved (an accepted, not-yet-triggered future need, matching this project's own established convention for such gaps).

### G. Full `TrunkPlacer` family — the 6 new kinds

**Shared height formula** (M5-B07's own already-HIGH-confidence formula, unchanged, reused by every kind below that carries the three base fields): `height = base_height + random.next_int_bounded(height_rand_a + 1) + random.next_int_bounded(height_rand_b + 1)` — TWO draws, `height_rand_a` before `height_rand_b`.

**Generalized return shape** (this blueprint's own additive extension, needed by every multi-branch placer below — M5-B07's own two placers return an implicit single foliage-attachment point, sufficient for their own straight/bending shapes; every kind in this section produces more than one canopy center):
```rust
/// One canopy-generation anchor a trunk placer hands back to the tree driver.
/// Mirrors vanilla's own `FoliageAttachment` record shape (moderate confidence on the
/// exact field set — `radius_offset` is carried for forward compatibility with a future
/// kind that needs it; every kind THIS blueprint implements sets it to `0`).
#[derive(Copy, Clone, Debug)]
pub struct FoliageAttachment {
    pub pos: rc_core::BlockPos,
    pub radius_offset: i32,
    /// True iff this attachment sits above a 2x2 trunk footprint (giant/mega_jungle/
    /// dark_oak) — the foliage placer must center its own canopy across all 4 trunk
    /// columns at this attachment, not just `pos` itself.
    pub double_trunk: bool,
}

/// Dispatches on `placer`'s own variant, returns `(actual_height, attachments)`.
/// `actual_height` is the number of Y-layers the trunk itself occupied (needed by the
/// caller to know where root/decorator bookkeeping ends and canopy begins).
pub fn place_trunk(
    placer: &TrunkPlacerJson,
    trunk_provider: &crate::decoration::providers::BlockStateProvider,
    origin: rc_core::BlockPos,
    world: &mut dyn crate::decoration::context::DecorationWorldAccess,
    resolver: &dyn crate::decoration::context::BlockStateResolver,
    random: &mut WorldgenRandom<AnyRandom>,
    log: &mut TreePlacementLog,
) -> (i32, Vec<FoliageAttachment>);
```

`TrunkPlacerJson` gains 6 new variants (M5-B07's already-shipped `StraightTrunkPlacer`/`BendingTrunkPlacer`/`Unsupported` arms are unchanged; `Unsupported` now only catches genuinely unrecognized future kinds, since all 8 confirmed vanilla trunk-placer types are covered between M5-B07 and this blueprint):

```rust
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TrunkPlacerJson {
    StraightTrunkPlacer { base_height: i32, height_rand_a: i32, height_rand_b: i32 },
    BendingTrunkPlacer { base_height: i32, height_rand_a: i32, height_rand_b: i32, #[serde(default)] bend_length: i32 },
    ForkingTrunkPlacer { base_height: i32, height_rand_a: i32, height_rand_b: i32 },
    GiantTrunkPlacer { base_height: i32, height_rand_a: i32, height_rand_b: i32 },
    MegaJungleTrunkPlacer { base_height: i32, height_rand_a: i32, height_rand_b: i32 },
    DarkOakTrunkPlacer { base_height: i32, height_rand_a: i32, height_rand_b: i32 },
    FancyTrunkPlacer { base_height: i32, height_rand_a: i32, height_rand_b: i32 },
    CherryTrunkPlacer {
        base_height: i32, height_rand_a: i32, height_rand_b: i32,
        branch_count: crate::data::IntProvider,
        branch_horizontal_length: crate::data::IntProvider,
        branch_start_offset_from_top: crate::data::IntProvider,
        branch_end_offset_from_top: crate::data::IntProvider,
    },
    #[serde(other)]
    Unsupported,
}
```

**`forking_trunk_placer`** (low-moderate confidence — this blueprint's own reconstruction of acacia's characteristic Y-fork silhouette). Height draw (2, shared formula). Place a straight single-log column from `origin.y` to `origin.y + height - 1` (fresh `trunk_provider` sample per log). Then TWO branches, each: `dir_idx = random.next_int_bounded(4)` (ONE draw), `branch_len = 1 + random.next_int_bounded(2)` (ONE draw, 1 or 2 logs) — branch 0's pair of draws strictly before branch 1's; each branch starts at the trunk top and steps `branch_len` times, each step advancing `(+dir_offset, +1 in Y)` (fresh sample per log); the branch's own final position becomes one `FoliageAttachment` (`radius_offset: 0, double_trunk: false`). TOTAL draws: `2 + 4 = 6`. No attachment at the bare trunk top itself (matches acacia's real "no leaves directly above the fork" silhouette).

**`giant_trunk_placer`** (HIGH confidence — this blueprint's designated exact hand-trace, Acceptance tests). Height draw (2, shared formula). Stamps a 2×2 log footprint (`(x,y,z)`, `(x+1,y,z)`, `(x,y,z+1)`, `(x+1,y,z+1)`, fresh sample per column per layer) for every `y` in `origin.y ..= origin.y + height - 1`. Zero further draws. Returns ONE `FoliageAttachment { pos: (origin.x, origin.y + height - 1, origin.z), radius_offset: 0, double_trunk: true }`.

**`mega_jungle_trunk_placer`** (moderate confidence this identity holds — this blueprint implements it as a direct call to the SAME internal 2×2-stamping routine `giant_trunk_placer` uses, differing only in whatever `base_height`/`height_rand_a`/`height_rand_b` the JSON config supplies; flagged explicitly as a simplification in case real vanilla's `MegaJungleTrunkPlacer` adds behavior `GiantTrunkPlacer` lacks). Same draw count and shape as `giant_trunk_placer`.

**`dark_oak_trunk_placer`** (low-moderate confidence). Height draw (2, shared formula). Stamps the same 2×2 footprint as `giant_trunk_placer` for `height` layers. THEN: `jut_dir = random.next_int_bounded(4)` (ONE draw) selects one of the 2×2 footprint's 4 corners to receive one extra outward-jutting log at the top layer (this blueprint's own reconstruction of dark oak's asymmetric canopy-widening silhouette). TOTAL draws: `2 + 1 = 3`. Returns TWO `FoliageAttachment`s at the top layer's `y + 1`, offset to the two corners diagonally opposite the jut direction (`double_trunk: true` on both) — this is what gives dark oak's canopy its characteristic wide, doubled-overlap flatness.

**`fancy_trunk_placer`** (LOW confidence — explicitly a bounded simplification of vanilla's real, genuinely complex "big oak" branch-slope algorithm, named as such rather than silently passed off as exact, matching M5-B07's own `lake` feature precedent). Height draw (2, shared formula). `trunk_height = (height as f32 * 0.618).floor() as i32` (golden-ratio split point, zero draws, deterministic). Straight single-log column from `origin.y` to `origin.y + trunk_height - 1`. `branch_count = 3 + random.next_int_bounded(3)` (ONE draw, 3–5 branches). For `i` in `0..branch_count` (ascending): `dir_idx = random.next_int_bounded(4)` (ONE draw), `branch_len = 2 + random.next_int_bounded(3)` (ONE draw, 2–4 logs) — branch `i`'s pair of draws strictly before branch `i+1`'s; each branch starts at `origin.y + trunk_height + i * (height - trunk_height) / branch_count.max(1)` (deterministic vertical spread across the upper crown, zero draws) and steps `branch_len` times diagonally `(+dir_offset, +1 in Y)`; each branch's final position is one `FoliageAttachment`. TOTAL draws: `2 + 1 + 2*branch_count`.

**`cherry_trunk_placer`** (moderate confidence on the extra field set — public datapack schema convention; low-moderate on the exact droop geometry). Height draw (2, shared formula). Straight single-log column to `height`. `branch_count_val = sample_int_provider(&config.branch_count, random)` (1+ draws). For each branch (ascending index): `start_offset = sample_int_provider(&config.branch_start_offset_from_top, random)` (1+ draws), `dir_idx = random.next_int_bounded(4)` (ONE draw), `horiz_len = sample_int_provider(&config.branch_horizontal_length, random)` (1+ draws) — branch starts at `origin.y + height - start_offset`, steps `horiz_len` times purely horizontally in `dir_idx` (Y held flat — this blueprint's own simplification of cherry's real gentle downward droop, `branch_end_offset_from_top` is sampled, 1+ draws, but only used to nudge the FINAL attachment's Y down by that amount, not every intermediate step); each branch's end is one `FoliageAttachment`.

### H. Full `FoliagePlacer` family — the 8 new kinds, and the multi-attachment driving loop

**Driving loop** (this blueprint's own restatement of vanilla's real "call once per attachment, sharing one continuing RNG stream" behavior): `tree::place` (Implementation steps) calls `place_foliage(&config.foliage_placer, &config.foliage_provider, &attachment, world, resolver, random)` once per `FoliageAttachment` the trunk placer returned, in the SAME order the trunk placer produced them — every attachment's own draws happen fully before the next attachment's own call begins (same depth-first discipline as `run_placement_chain`, Context §0).

```rust
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FoliagePlacerJson {
    BlobFoliagePlacer { radius: crate::data::IntProvider, offset: crate::data::IntProvider, height: i32 },
    SpruceFoliagePlacer { radius: crate::data::IntProvider, offset: crate::data::IntProvider, trunk_height: crate::data::IntProvider },
    PineFoliagePlacer { radius: crate::data::IntProvider, offset: crate::data::IntProvider, height: i32 },
    AcaciaFoliagePlacer { radius: crate::data::IntProvider, offset: crate::data::IntProvider },
    BushFoliagePlacer { radius: crate::data::IntProvider, offset: crate::data::IntProvider, height: i32 },
    FancyFoliagePlacer { radius: crate::data::IntProvider, offset: crate::data::IntProvider, height: i32 },
    JungleFoliagePlacer { radius: crate::data::IntProvider, offset: crate::data::IntProvider, height: i32 },
    MegaPineFoliagePlacer { radius: crate::data::IntProvider, offset: crate::data::IntProvider, height: i32, crown_height: crate::data::IntProvider },
    DarkOakFoliagePlacer { radius: crate::data::IntProvider, offset: crate::data::IntProvider },
    CherryFoliagePlacer { radius: crate::data::IntProvider, offset: crate::data::IntProvider, height: i32 },
    #[serde(other)]
    Unsupported,
}
```

**`pine_foliage_placer`** (moderate confidence). `radius0 = sample_int_provider(&radius, random)`, `offset0 = sample_int_provider(&offset, random)` (2 draws total). For `layer in 0..height` (Y descending from `attachment.pos.y - offset0`): `layer_radius = if layer % 2 == 0 { radius0 } else { (radius0 - 1).max(0) }` (deterministic alternation, zero draws); fill a SQUARE (not disk — this blueprint's own moderate-confidence differentiator from Blob's circular corner-skip shape, matching pine's blockier public silhouette) of `-layer_radius..=layer_radius` in both X/Z where the target is air-or-replaceable, fresh `foliage_provider` sample per block. TOTAL draws: 2.

**`acacia_foliage_placer`** (moderate confidence). `radius0`, `offset0` (2 draws). ONE flat disk layer at `attachment.pos.y - offset0`, radius `radius0`, corners (`|dx|==radius0 && |dz|==radius0`) always skipped deterministically (no draw — unlike Blob's own probabilistic corner-skip). TOTAL draws: 2.

**`bush_foliage_placer`** (HIGH confidence — a direct, verbatim reuse of M5-B07's own already-shipped `place_blob_foliage` routine; a real, well-documented vanilla class-hierarchy fact, not a guess). Calls M5-B07's existing Blob algorithm unmodified. TOTAL draws: identical to Blob (2, plus one `next_int_bounded(2)` per exact-corner candidate).

**`jungle_foliage_placer`** (moderate confidence). Identical to Blob EXCEPT the topmost layer (`layer == height`) uses `radius0 - 1` instead of `radius0` (zero extra draws — a deterministic per-layer override); every other layer, and the per-corner skip draw, are exactly Blob's own shape. TOTAL draws: 2 plus one `next_int_bounded(2)` per exact-corner candidate (unchanged from Blob).

**`fancy_foliage_placer`** (moderate confidence). `radius0`, `offset0` (2 draws). For `layer in 0..=height` (Y ascending from `attachment.pos.y - offset0`): `layer_radius = radius0 - ((layer - height/2).abs() / 2)` (a tapering, roughly-spherical profile, zero draws, deterministic); fill positions where `dx*dx + dz*dz <= layer_radius*layer_radius + 1` (the `+1` softening the circle's edge) and air-or-replaceable, fresh sample per block. TOTAL draws: 2.

**`mega_pine_foliage_placer`** (moderate confidence). `radius0`, `offset0`, `crown = sample_int_provider(&crown_height, random)` (3 draws). The top `crown` layers use `radius0` unconditionally (a denser cap); layers below alternate `radius0`/`radius0-1` exactly as `pine_foliage_placer`. TOTAL draws: 3.

**`dark_oak_foliage_placer`** (moderate confidence — called once per each of the trunk placer's TWO attachments, per the driving loop). `radius0`, `offset0` (2 draws PER call/attachment). A flat two-layer disk (`attachment.pos.y` and `attachment.pos.y - 1`) of radius `radius0 + 1`, corners always skipped deterministically (like Acacia). TOTAL draws per attachment: 2 (so 4 total across both of dark oak's own attachments, in the order the trunk placer returned them).

**`cherry_foliage_placer`** (low-moderate confidence — this blueprint's own "sparse variant of Blob" reconstruction). `radius0`, `offset0` (2 draws). Same layer/disk shape as Blob, but EVERY non-corner candidate position (not just exact corners) additionally draws `thin_roll = random.next_int_bounded(3)` (ONE draw per candidate) and is skipped unless `thin_roll == 0` — giving cherry's canopy its characteristic sparse, delicate look. Exact corners keep Blob's own separate `next_int_bounded(2)`-gated skip (both draws happen for a corner candidate: corner-skip roll first, then — only if the corner survives — the general thinning roll).

### I. Out-of-scope kinds, restated (End-exclusive, 5 kinds)

`chorus_plant`, `end_platform`, `end_spike`, `end_island`, `end_gateway` are NOT dispatched by `place_configured_feature_vegetation` — a `ConfiguredFeature` naming any of these five `feature_type` strings falls all the way through to `place_configured_feature_all`'s own "unrecognized kind, documented `debug`-logged no-op" default (Context §0), identical in observable behavior to every other not-yet-implemented kind, but for a structurally different reason (dimension-scope exclusion, not an implementation gap) — restated here so a future reader never mistakes "the End is out of scope" for "nobody got around to it yet." `void_start_platform`, despite its similar End-adjacent name, is **not** in this list — M5-B12c already implements it (its own Context §L.2), since it turns out to be usable outside the End too. Revisit only if a future milestone brings the End dimension into GEN-D1's scope.

### J. Config structs and algorithms — every terminal kind this blueprint owns (17)

Every kind below is a `Configuration` struct plus one `pub fn place(config: &..Configuration, origin: rc_core::BlockPos, world: &mut dyn DecorationWorldAccess, resolver: &dyn BlockStateResolver, props: &dyn BlockPropertyResolver, random: &mut WorldgenRandom<AnyRandom>)`, matching M5-B07's/M5-B12's own established per-kind file convention exactly (Deliverables lists the concrete file placement). The fixed direction table from Context §F applies identically here.

**`fallen_tree`** (`FallenTreeConfiguration { trunk_provider: BlockStateProvider, log_length: crate::data::IntProvider }`, low-moderate confidence). `dir_idx = random.next_int_bounded(4)` (ONE draw); `len = sample_int_provider(&config.log_length, random)` (1+ draws); if the block below `origin` is solid, place `len` consecutive `trunk_provider` samples (fresh sample per log, correct horizontal log-axis property implied by `dir_idx`'s axis) walking from `origin` in the `dir_idx` direction.

**`huge_fungus`** (`HugeFungusConfiguration { valid_base_state: crate::data::BlockStateSpec, stem_state: crate::data::BlockStateSpec, hat_state: crate::data::BlockStateSpec, decor_state: crate::data::BlockStateSpec, #[serde(default)] planted: bool }`, low-moderate confidence). Gate: block below `origin` must resolve to `config.valid_base_state` (skip via `resolver`-equality check on the block id, zero draws) unless `config.planted` (planted variants skip the base-block gate, matching vanilla's own "already growing from a sapling-equivalent" case). `height = 5 + random.next_int_bounded(3)` (ONE draw, 5–7). Straight `stem_state` column for `height` layers (zero further draws — `stem_state` is a fixed `BlockStateSpec`, not a `BlockStateProvider`, so every layer resolves to the identical state). "Hat": a `blob_foliage_placer`-style disk (radius 3, zero further RNG since radius/offset are this blueprint's own fixed constants rather than sampled) of `hat_state` centered at the stem top, EXCEPT each hat block independently rolls `random.next_int_bounded(5) == 0` (ONE draw per hat-candidate block) to instead place `decor_state` (shroomlight-equivalent decoration).

**`huge_red_mushroom` / `huge_brown_mushroom`** (SHARED `HugeMushroomConfiguration { cap_provider: BlockStateProvider, stem_provider: BlockStateProvider, foliage_radius: i32 }`, one Rust function `place_huge_mushroom(is_red: bool, ..)` dispatched from both `feature_type` strings — task-required "exact cap math," LOW-MODERATE confidence on the cap geometry itself, restated explicitly as an approximation rather than a byte-exact claim). `height = 4 + random.next_int_bounded(3)` (ONE draw, 4–6 — this blueprint's own regression-pinned constant, Acceptance tests). Straight `stem_provider` column for `height - 1` layers (leaving the top layer for the cap to own; zero further draws, fresh sample per layer). Cap, deterministic (zero further RNG — vanilla's real mushroom cap shape is a fixed function of height, not per-block random): for the TOP 3 Y-layers (`height-3 ..= height-1`, clamped at 0), each at `radius = config.foliage_radius` for `is_red` (a smaller, domed profile — this blueprint tapers by 1 per layer descending FROM the top: top layer radius `radius-1`, then `radius`, then `radius` again) or a FLAT single-layer `radius+1` disk with no tapering for `is_brown`; every cap position within the layer's own radius (Euclidean, `dx*dx+dz*dz <= r*r`) gets `cap_provider`; the outermost ring's 4 exact corners are additionally notched (skipped) on the TOP layer only for `is_red` (the well-known rounded-cap silhouette), never notched for `is_brown` (flat table-top silhouette) — this notch behavior is this blueprint's own concrete, testable, but NOT vanilla-byte-verified approximation of the real distinguishing visual feature between the two mushroom kinds.

**`vegetation_patch` / `waterlogged_vegetation_patch`** (SHARED `VegetationPatchConfiguration { ground_state: BlockStateProvider, vegetation_feature: crate::data::ResourceLocation, replaceable_tag: String, depth: crate::data::IntProvider, #[serde(default)] extra_bottom_block_chance: f32, vertical_range: i32, xz_radius: crate::data::IntProvider, #[serde(default)] extra_edge_column_chance: f32 }`, one function `place_vegetation_patch(waterlogged: bool, ..)` dispatched from both `feature_type` strings — this is the ONLY kind in this blueprint that needs the extended `(data, ctx)` parameters, since it recurses into `vegetation_feature` (Context §0) — LOW-MODERATE confidence, this blueprint's own reconstruction). `radius = sample_int_provider(&config.xz_radius, random)` (1+ draws). For `dx in -radius..=radius, dz in -radius..=radius` where `dx*dx+dz*dz <= radius*radius` (ascending `dx` then `dz`, matching every other modifier's own established column-scan order convention): if the column is exactly at the boundary (`dx*dx+dz*dz > (radius-1)*(radius-1)`), draw `edge_roll = random.next_float()` (ONE draw, conditional on being a boundary column) and skip the column if `edge_roll >= extra_edge_column_chance`; else (interior column) no edge draw. For each surviving column: scan down from `origin.y + config.vertical_range` for the first position whose block matches `props.matches_tag(state, &config.replaceable_tag)` with solid ground beneath (zero draws); for `waterlogged`, additionally require `props.is_still_water` at that position (skip the column, zero draws, if not underwater); place `sample_block_state_provider(&config.ground_state, random, resolver)` there (fresh sample); draw `bottom_roll = random.next_float()` (ONE draw); if `bottom_roll < config.extra_bottom_block_chance`, ALSO place a fresh `ground_state` sample one block below; re-enter `data.placed_features[&config.vegetation_feature]`'s own full chain (Context §0's re-entry shape) at the surface position.

**`vines`** (`VineConfiguration {}`, zero fields, LOW-MODERATE confidence — the ordinary overworld climbing vine, not to be confused with M5-B12's own, entirely separate `multiface_growth`/glow-lichen kind). For `dx in -4..=4, dz in -4..=4` (ascending, fixed 9×9 footprint at `origin.y`): if `world.get_block(origin+(dx,0,dz))` is air, for `dir_idx in 0..4` (fixed N,E,S,W order): if the neighbor in that direction is solid, draw `roll = random.next_bool()` (ONE draw PER direction PER air candidate — every one of the 4 directions is always rolled once the position itself is air, regardless of whether an earlier direction already succeeded, since a position can carry vines on multiple faces simultaneously in vanilla); place `vine[facing=dir]` iff `roll`.

**`weeping_vines` / `twisting_vines`** (both `VineConfiguration {}`-shaped — no config, vanilla's own real classes hardcode their block ids exactly as this blueprint's own dispatch hardcodes which native block-id pair to use per `feature_type`, faithfully reproducing vanilla's own "not JSON-driven for these two" architecture rather than deviating from it; LOW-MODERATE confidence on the height-roll constants). `height = 8 + random.next_int_bounded(4)` (ONE draw, this blueprint's own best-effort constant pair, 8–11 for weeping [ceiling-hanging, grows DOWN], mirrored for twisting [floor-based, grows UP] with the same draw shape). Place `height - 1` "plant" body-block segments plus one distinct "tip" block at the far end, walking down (weeping) or up (twisting) from `origin`, stopping early on hitting a non-air block.

**`bamboo`** (`BambooConfiguration { probability: f32 }`, LOW-MODERATE confidence). `roll = random.next_float()` (ONE draw); if `roll >= probability`, no-op. Else: `stalk_height = 5 + random.next_int_bounded(12)` (ONE draw, 5–16, this blueprint's own best-effort constants). Place `stalk_height` vertical bamboo segments; the bottom third get no leaves (`leaves=none`), the middle third `leaves=large`, the top third `leaves=small` (deterministic 3-zone mapping, zero further draws).

**`nether_forest_vegetation`** (`NetherForestVegetationConfiguration { state_provider: BlockStateProvider, spread_width: i32, spread_height: i32 }`, LOW-MODERATE confidence — this blueprint's own `tries = spread_width * spread_width` reconstruction, a common "quadratic in radius" convention matching several other patch-shaped vanilla features, not independently verified for this specific kind). For `i in 0..tries` (ascending): `dx = random.next_int_bounded(spread_width*2+1) - spread_width`, `dz = random.next_int_bounded(spread_width*2+1) - spread_width`, `dy = random.next_int_bounded(spread_height*2+1) - spread_height` (THREE draws, X then Z then Y — same order convention as `random_offset`/`random_patch`, M5-B07 Context §G.11/§N.6); candidate `= origin + (dx,dy,dz)`; if air-or-replaceable with solid, valid-floor-tag-matching ground beneath, place `sample_block_state_provider(&config.state_provider, random, resolver)` (fresh sample per successful candidate).

**`seagrass`** (`ProbabilityConfiguration { probability: f32 }`, moderate confidence). Gate: `origin` must be `props.is_still_water`, zero draws. `is_tall = random.next_double() < 0.3` (ONE draw — this blueprint's own best-effort constant for the short/tall split, distinct from the config's own `probability`, which the PLACEMENT-modifier chain, not this terminal feature, is assumed to have already applied via `noise_threshold_count`/`count` upstream per M5-B07's own established modifier-vs-feature-probability split). If `is_tall`: place `seagrass[half=lower]` at `origin`, `seagrass[half=upper]` at `origin+(0,1,0)` iff that position is also water; else place a single non-tall `seagrass` block at `origin`.

**`kelp`** (no config, `NoOpConfiguration`-shaped, moderate confidence on the nested-draw height formula — this blueprint's own extrapolation of the general zero-biased nested-`nextInt` pattern vanilla is known to use elsewhere (e.g. `CaveWorldCarver`'s cave-count formula, `docs/research/mc-26.2/05-worldgen.md` §5: `nextInt(nextInt(nextInt(15)+1)+1)` — that specific formula is 3-level/bound-15 and governs an unrelated subsystem, cave count, not kelp height; the research corpus does not document kelp's own 2-level/bound-10 formula, restated here as this blueprint's own best-effort match, not a documented precedent). Gate: `origin` water with solid floor beneath, zero draws. `height = 1 + random.next_int_bounded(random.next_int_bounded(10) + 1)` (TWO draws, outer bound depends on the inner draw's own result — the inner draw happens FIRST). Place `height - 1` `kelp_plant` segments plus one `kelp` tip block, stacking upward from `origin`, stopping early on the first non-water position.

**`sea_pickle`** (`SeaPickleConfiguration { count: crate::data::IntProvider }` — this blueprint's designated second exact hand-trace, Acceptance tests). `raw = sample_int_provider(&config.count, random)` (1+ draws, provider-dependent — for the hand-trace's own `IntProvider::Uniform{min:0,max:3}` config this becomes exactly ONE `next_int_bounded`-derived draw, Context §0's `Uniform` row). `pickles = (1 + raw).clamp(1, 4)`; place `sea_pickle[pickles=pickles]` at `origin` iff the position is air-or-water with a solid block beneath.

**`coral_tree` / `coral_mushroom` / `coral_claw`** (SHARED `CoralFeatureConfiguration { state: crate::data::BlockStateSpec }`, one function per shape, LOW-MODERATE confidence on every geometric constant). Gate, shared by all three: `origin` must be `props.is_still_water` with a solid floor beneath, zero draws; if the gate fails, no-op (this blueprint's own stand-in for vanilla's real "dead coral" fallback, which needs a second, differently-colored block state this blueprint's own single-`state`-field config does not carry — a named, bounded simplification). **`coral_tree`**: `trunk_len = 1 + random.next_int_bounded(3)` (ONE draw, 1–3); straight `state`-block column for `trunk_len` layers; then TWO diagonal branches (fixed shape, zero further draws — each branch is exactly 2 blocks, stepping up+outward in a FIXED pair of opposite horizontal directions rather than randomly chosen, this blueprint's own deliberate simplification to keep the RNG-order surface small and testable). **`coral_mushroom`**: a small deterministic rounded blob (radius 2, 2 Y-layers, zero draws — same shape family as the huge-mushroom cap's own deterministic-given-height geometry, scaled down). **`coral_claw`**: `arm_count = 2 + random.next_int_bounded(2)` (ONE draw, 2–3); each arm steps 2 blocks outward horizontally in an evenly-spaced fixed direction (`arm_index * (360/arm_count)`, zero further draws) then 1 block up.

### K. Java → Rust porting-pitfall checklist (this blueprint's own additions to M5-B07's Context §P)

1. **`beehive`/`cocoa`/`trunk_vine`/`vegetation_patch`'s edge-column roll are all CONDITIONAL draws** — the direction/placement draw only happens after (and only if) the gating probability roll already succeeded; an implementation that always draws both, in either order, desyncs the very next feature's own RNG stream (the identical hazard M5-B07 Context §N.1 already named for `ore`'s own discard-on-air-exposure roll).
2. **`kelp`'s nested `nextInt(nextInt(...))` draws the INNER call first** — the outer bound is not known until the inner draw completes; reversing the order (or memoizing/reusing a value) desyncs immediately.
3. **The multi-attachment tree driving loop is depth-first per attachment** (Context §H) — exactly the same hazard M5-B07 Context §F names for placement modifiers, restated here for the trunk-placer → foliage-placer boundary specifically: every attachment's own foliage draws complete before the next attachment's own call begins.
4. **`FoliageAttachment` is a genuinely new return shape** (Context §G) — a trunk placer that returns a single position (as M5-B07's own two kinds effectively did) where this blueprint's kinds need multiple silently drops canopy coverage at every attachment but the first, producing a plausible-but-wrong-looking tree with no compiler signal.
5. **This blueprint's own `root_placer` (Context §E) and M5-B12c's own `root_system` Feature kind are different mechanisms with a confusingly similar name** — never conflate the two when reading either blueprint (Context §E's own explicit note).
6. **`driver.rs`'s per-feature call site is edited by three blueprints in sequence** (M5-B07 defines it, M5-B12e changes it, this blueprint changes it again) — apply them in that order; this blueprint's own Deliverables text for that file is the final, correct end state (Context §D).

## Deliverables

### `crates/worldgen/src/decoration/features/tree.rs` (MODIFY — M5-B07 file, confirmed untouched by the M5-B12 family)

Extends `TrunkPlacerJson` with 6 new variants (Context §G), `FoliagePlacerJson` with 8 new variants (Context §H), adds `RootPlacerJson` (Context §E), `FoliageAttachment` (Context §G), extends `TreeConfiguration` with `root_placer`/`decorators` fields (Context §E), adds `place_trunk`/`place_foliage`/`place_root_system` free functions, imports `TreePlacementLog` from the new `vegetation` module (below — defined there since tree decorators are its primary consumer, populated here), and rewrites `place`'s internal body (public signature UNCHANGED from M5-B07: `pub fn place(config: &TreeConfiguration, origin: rc_core::BlockPos, world: &mut dyn DecorationWorldAccess, resolver: &dyn BlockStateResolver, props: &dyn BlockPropertyResolver, random: &mut WorldgenRandom<AnyRandom>)`) to: (1) run the root placer if present (Context §E), adjusting the trunk's own effective origin; (2) call `place_trunk`, accumulating `TreePlacementLog`; (3) call `place_foliage` once per returned `FoliageAttachment`, accumulating leaf positions into the same log; (4) run every `config.decorators` entry in list order via `apply_tree_decorator`.

### `crates/worldgen/src/decoration/vegetation/mod.rs` (NEW — mirrors the M5-B12 family's own `underground/mod.rs` layout convention)

```rust
//! This blueprint's own module: the 17 vegetation-classified `Feature` kinds M5-B07
//! Context §M individually named and deferred, owned by this blueprint per the
//! ownership audit in `blueprints/M5/M5-B00-index.md` (Context §A/§B). See this
//! module's owning blueprint (`M5-B11`) for the full derivation.

pub mod tree_decorators;
pub mod fallen_tree;
pub mod mushroom;
pub mod vegetation_patch;
pub mod vines;
pub mod bamboo;
pub mod nether_vegetation;
pub mod ocean_vegetation;

pub use tree_decorators::{apply_tree_decorator, TreeDecoratorJson, TreePlacementLog};

/// Context §F — shared by every file in this module.
pub fn dir_offset(dir_idx: i32) -> (i32, i32);
/// Context §F — shared by every file in this module.
pub fn dir_name(dir_idx: i32) -> &'static str;

/// Context §D — the composed dispatcher, `driver.rs`'s own real call site (below).
#[allow(clippy::too_many_arguments)]
pub fn place_configured_feature_vegetation(
    feature: &crate::data::ConfiguredFeature,
    origin: rc_core::BlockPos,
    world: &mut dyn crate::decoration::context::DecorationWorldAccess,
    resolver: &dyn crate::decoration::context::BlockStateResolver,
    props: &dyn crate::decoration::context::BlockPropertyResolver,
    random: &mut crate::random::WorldgenRandom<crate::noise::AnyRandom>,
    data: &crate::data::WorldgenData,
    ctx: &crate::decoration::modifiers::PlacementCtx,
    bridge: Option<&crate::decoration::underground::UndergroundFeatureContext>,
);
```

### `crates/worldgen/src/decoration/vegetation/tree_decorators.rs` (NEW)

`TreePlacementLog`, `TreeDecoratorJson`, `apply_tree_decorator`, and the 6 implemented decorator algorithms (Context §F).

### `crates/worldgen/src/decoration/vegetation/{fallen_tree,mushroom,vegetation_patch,vines,bamboo,nether_vegetation,ocean_vegetation}.rs` (NEW, one file per family)

Each exposes its own `Configuration` struct(s) and `pub fn place(..)` per Context §J's exact shapes: `fallen_tree.rs` (`fallen_tree`), `mushroom.rs` (`huge_fungus`, `huge_red_mushroom`/`huge_brown_mushroom` sharing `place_huge_mushroom(is_red: bool, ..)`), `vegetation_patch.rs` (`vegetation_patch`/`waterlogged_vegetation_patch` sharing `place_vegetation_patch(waterlogged: bool, ..)`, the one file in this module needing the extra `(data, ctx)` parameters), `vines.rs` (`vines`, `weeping_vines`/`twisting_vines` sharing a body-and-tip helper), `bamboo.rs` (`bamboo`), `nether_vegetation.rs` (`nether_forest_vegetation`), `ocean_vegetation.rs` (`seagrass`, `kelp`, `sea_pickle`, `coral_tree`/`coral_mushroom`/`coral_claw`).

### `crates/worldgen/src/decoration/mod.rs` (MODIFY — M5-B07 file, one new module line, independent of M5-B12a's own identically-shaped addition to the same file)

```rust
pub mod vegetation;
```

### `crates/worldgen/src/decoration/driver.rs` (MODIFY — the final, composed form of this file's one per-feature call site)

**This blueprint's own text supersedes M5-B12e's own already-specified version of the identical line** (Context §D's ordering note; M5-B12e Context §S's own text is the intermediate state, not the final one). `decorate_chunk`'s own signature keeps the `bridge: Option<&UndergroundFeatureContext>` trailing parameter M5-B12e already added (unchanged, reused, not modified again). Its per-feature call site becomes:
```rust
crate::decoration::vegetation::place_configured_feature_vegetation(
    &data.configured_features[&placed.feature], pos, world, resolver, props, random, data, &ctx, bridge,
)
```
replacing M5-B12e's own `place_configured_feature_all(..., &ctx, bridge)` call (which remains fully reachable, one layer further out, for every kind this blueprint does not itself claim, with `ctx` still correctly threaded through it). No other line of M5-B07's or the M5-B12 family's own `decorate_chunk` algorithm (seeding, possible-biomes computation, per-step reachable-set union, global-index sort) changes.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary** (TEST-D45/D46, restated from M5-B07/M5-B12 unchanged): every new file above, and every modification to an M5-B07/M5-B12 file, is committed with every new/changed function body `todo!()`-stubbed (full signatures, full derives) in this first changeset, alongside every test file below. The implementation changeset fills bodies only.

### `crates/worldgen/tests/vegetation_trunk_foliage.rs`

1. `giant_trunk_placer_hand_trace` — `WorldgenRandom::new(AnyRandom::new_legacy(777))`, `TrunkPlacerJson::GiantTrunkPlacer{base_height:5, height_rand_a:3, height_rand_b:2}`, `origin=(0,64,0)`: hand-traced against M5-B01's own published `next_int_bounded` formula (rejection loop, since `4` and `3` are not powers of two — computed once by hand for this blueprint's own derivation pass): draw 1 (`next_int_bounded(4)`) = `2`, draw 2 (`next_int_bounded(3)`) = `0`, `height = 5+2+0 = 7`. Assert `place_trunk` returns `(7, vec![FoliageAttachment{pos:(0,70,0), radius_offset:0, double_trunk:true}])` and that `world`'s recorded `set_block` calls are exactly the 28 positions `{(0,y,0),(1,y,0),(0,y,1),(1,y,1) : y in 64..=70}` (order: this blueprint's own `place_trunk` implementation's own deterministic column/layer iteration order — asserted as a literal expected `Vec<BlockPos>`, a regression pin on THIS blueprint's own algorithm, not a vanilla-verified value, per this project's established convention).
2. `forking_trunk_placer_consumes_six_draws` — fixed seed; instrument draw-count via RNG-state comparison against an independently-computed 6-draw reference sequence (2 height + 2×2 branch draws); assert exactly 6 draws consumed and exactly 2 `FoliageAttachment`s returned, at different positions.
3. `mega_jungle_trunk_placer_matches_giant_shape` — same fixed seed and config field values fed to both `GiantTrunkPlacer` and `MegaJungleTrunkPlacer`; assert byte-identical resulting block sets and attachments (proves the shared-routine claim, Context §G).
4. `dark_oak_trunk_placer_returns_two_double_trunk_attachments` — structural: both returned `FoliageAttachment`s have `double_trunk: true`, are at the same Y, and are at distinct X/Z positions.
5. `fancy_trunk_placer_branch_count_in_range` — for 200 fixed seeds, `branch_count` (inferred from the returned attachment count) is always in `[3,5]`.
6. `cherry_trunk_placer_attachment_count_matches_branch_count_provider` — `branch_count: IntProvider::Constant(4)`; assert exactly 4 `FoliageAttachment`s returned.
7. `bush_foliage_placer_matches_blob_exactly` — same fixed seed, same `radius`/`offset`/`height` config fed to both `place_blob_foliage` (M5-B07's own already-shipped function) and `bush_foliage_placer`'s dispatch; assert byte-identical output (Context §H's HIGH-confidence reuse claim).
8. `jungle_foliage_placer_top_layer_narrower` — fixed seed, `radius: Constant(3)`, `height: 4`; assert the topmost layer's placed-block max horizontal distance from center is `2` while every lower layer's is `3`.
9. `acacia_dark_oak_foliage_corners_always_skipped` — for both kinds, across 50 fixed seeds, no placed block is ever at an exact corner of its own layer's bounding square.
10. `multi_attachment_foliage_driving_loop_is_depth_first_per_attachment` — a synthetic 3-attachment trunk result fed through `AcaciaFoliagePlacer` (2 draws/attachment); assert the SAME final RNG state results whether processed via this blueprint's own driving loop or via 3 independently-chained manual calls in attachment order (proves no interleaving, Context §K.3).

### `crates/worldgen/tests/vegetation_root_and_decorators.rs`

1. `mangrove_root_placer_requires_correct_soil` — `FakeWorld` with a non-mud block beneath `origin`; assert `TreeConfiguration::place` (with a `root_placer` present) writes ZERO blocks anywhere (the whole-feature no-op simplification, Context §E).
2. `mangrove_root_placer_walks_down_from_mud` — mud beneath `origin`, fixed seed, `max_root_width:1, max_root_length:5, random_skew_chance:0.0` (skew never triggers, deterministic straight-down walk); assert exactly the column `(origin.x, origin.y-1..=origin.y-5, origin.z)` (or fewer if a non-replaceable block is hit first) receives `root_provider` samples.
3. `beehive_direction_draw_is_conditional_on_probability_roll` — `probability: 0.0` (roll always fails since `next_float() >= 0.0` is never true); assert RNG state after the call reflects EXACTLY one draw consumed (the probability roll), never two.
4. `beehive_places_when_leaf_and_air_present` — `probability: 1.0` (always succeeds); `FakeWorld` with a leaf position directly above the computed candidate; assert a `bee_nest` block is placed with the correct `facing` property matching the drawn direction.
5. `trunk_vine_and_leave_vine_share_draw_shape` — same fixed seed, same synthetic single-log/single-leaf `TreePlacementLog` fed to both decorators; assert identical draw-count behavior (4 conditional draws, one per direction, only for air neighbors).
6. `cocoa_second_draw_conditional_on_first` — analogous to beehive's test 3, for `cocoa`.
7. `unsupported_decorator_kind_is_documented_no_op` — `TreeDecoratorJson::Unsupported` (via an unrecognized `type` string); assert `apply_tree_decorator` writes zero blocks and consumes zero draws, does not panic.

### `crates/worldgen/tests/vegetation_terminal_features.rs`

1. `sea_pickle_hand_trace` — `WorldgenRandom::new(AnyRandom::new_legacy(42))`, `SeaPickleConfiguration{count: IntProvider::Uniform{min:0,max:3}}` (a `Uniform` provider chosen specifically to land on exactly ONE draw via `next_int_between_inclusive`, Context §0): hand-traced draw = `2` (computed once by hand against M5-B01's own published `next_int_bounded` rejection-loop formula for this exact seed/bound), `pickles = (1+2).clamp(1,4) = 3`; assert `sea_pickle[pickles=3]` is placed at `origin` (with a `FakeWorld` pre-seeded so the position/floor gate passes).
2. `kelp_inner_draw_happens_before_outer_bound_is_known` — instrumented via a `WorldgenRandom` wrapper that records each `next_int_bounded` call's OWN `bound` argument in call order; assert the recorded sequence is `[10, X]` where `X` is whatever the FIRST draw's own result was `+1` (proving the inner `nextInt(10)` call's result determines the second call's bound, Context §K.2).
3. `huge_mushroom_red_vs_brown_cap_shape_differs` — same fixed seed, same `HugeMushroomConfiguration`, dispatched once as `is_red:true` and once `is_red:false`; assert the red cap's top-layer corner positions are absent from the placed-block set while the brown cap's are present (the one concretely testable geometric distinction this blueprint commits to, Context §J).
4. `vegetation_patch_recurses_into_nested_placed_feature` — mirrors M5-B07's own `random_patch` recursion test exactly (a trivial `simple_block`-configured nested feature); asserts the nested feature's own block actually appears at the computed surface position, not merely that candidacy was validated.
5. `waterlogged_vegetation_patch_requires_underwater_column` — same fixture as test 4 but with a non-water column; assert zero placements (the `waterlogged`-only gate, Context §J).
6. `weeping_vines_and_twisting_vines_grow_opposite_directions` — same fixed seed and height-draw outcome fed to both; assert `weeping_vines`' placed Y range descends from `origin.y` while `twisting_vines`' ascends.
7. `bamboo_probability_gate_is_the_only_draw_on_failure` — `probability: 0.0`; assert exactly one draw consumed (the gate roll) and zero blocks placed.
8. `coral_family_no_ops_when_not_underwater` — for all three coral kinds, a non-water `FakeWorld` column; assert zero placements and (for `coral_tree`/`coral_claw`) zero RNG draws consumed (the gate check precedes every draw).
9. `nether_forest_vegetation_try_count_is_spread_width_squared` — `spread_width: 3`; instrument candidate-position draw triples; assert exactly 9 candidates are evaluated.
10. `fallen_tree_direction_draw_precedes_length_draw` — fixed seed; instrument draw order; assert the direction draw's own RNG state snapshot precedes the length draw's.

### `crates/worldgen/tests/vegetation_dispatch_composition.rs`

1. `vegetation_dispatcher_handles_own_kinds_directly` — for each of the 17 kinds in Context §B's own-kind list, a minimal `ConfiguredFeature` fixture; assert `place_configured_feature_vegetation` produces the identical world state this blueprint's own individual `place` function would for the same input.
2. `vegetation_dispatcher_falls_through_to_underground_dispatcher` — a `ConfiguredFeature{feature_type: "minecraft:no_op", ..}` (an M5-B12e-owned kind); assert `place_configured_feature_vegetation`'s own output is byte-identical to calling `crate::decoration::underground::place_configured_feature_all` directly with the SAME `ctx`/`data`/`bridge` arguments — proving the fall-through is a pure pass-through, not a re-implementation.
3. `vegetation_dispatcher_forwards_ctx_to_a_ctx_dependent_fallthrough_kind` — a `ConfiguredFeature{feature_type: "minecraft:random_selector", ..}` (an M5-B12e-owned kind whose own algorithm re-enters `run_placement_chain` via `ctx`, Context §0), whose nested `entry.feature` resolves to a `simple_block`-configured `placed_feature`; assert the delegated block is actually written at `origin` when reached THROUGH `place_configured_feature_vegetation`'s own fallthrough path — proving `ctx` survives the fallthrough call intact, not merely that the call compiles. This is the exact class of gap a byte-identical-to-`no_op` test (test 2) cannot catch, since `no_op` never touches `ctx` at all.
4. `end_exclusive_kinds_fall_all_the_way_through_to_documented_no_op` — for each of the 5 out-of-scope `feature_type` strings (Context §I), assert `place_configured_feature_vegetation` writes zero blocks and does not panic.
5. `chorus_plant_mutual_gap_is_a_safe_no_op` — `feature_type: "minecraft:chorus_plant"` specifically (Context §A's own flagged gap); assert the SAME safe no-op behavior as test 4, confirming the gap is harmless in practice.

## Implementation steps

1. **`tree.rs` extension** (Context §G/§H/§E/§F). Add `FoliageAttachment`, the 6 new `TrunkPlacerJson` variants + `place_trunk`'s dispatch, the 8 new `FoliagePlacerJson` variants + `place_foliage`'s dispatch, `RootPlacerJson` + root-placement logic, `TreeConfiguration`'s two new fields. Rewrite `place`'s body per Deliverables' description, public signature unchanged. Observable: `vegetation_trunk_foliage.rs` and `vegetation_root_and_decorators.rs`'s root-placer tests pass.
2. **`vegetation/tree_decorators.rs`** (Context §F). `TreePlacementLog`, `TreeDecoratorJson`, `apply_tree_decorator`, the 6 implemented decorators. Observable: `vegetation_root_and_decorators.rs`'s decorator tests pass.
3. **`vegetation/{fallen_tree,mushroom,vegetation_patch,vines,bamboo,nether_vegetation,ocean_vegetation}.rs`** (Context §J). Observable: `vegetation_terminal_features.rs` passes.
4. **`vegetation/mod.rs`** — `dir_offset`/`dir_name`, `place_configured_feature_vegetation`'s dispatch (this blueprint's 17 arms, else fall through to `crate::decoration::underground::place_configured_feature_all`). Observable: `vegetation_dispatch_composition.rs` passes.
5. **`decoration/mod.rs`, `decoration/driver.rs`** — the two additive edits (Deliverables). Observable: `cargo build -p rc-worldgen` succeeds with zero `todo!()` remaining in this blueprint's own files, full workspace builds.
6. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` all exit 0.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding**, identical discipline to M5-B07/M5-B12: every test file above is committed first, verbatim, alongside `todo!()`-stubbed sources; the implementation changeset fills bodies only.

(b) **No new `[workspace.dependencies]` entry and no `Cargo.toml` change** on `rc-worldgen` — every type this blueprint uses is already reachable through M5-B01/M5-B02/M5-B03/M5-B07's and the M5-B12 family's existing edges.

(c) **No Mojang or third-party reimplementation source is consulted.** Every algorithm in this blueprint's own Context section is this blueprint's own restatement from public, general architectural knowledge and `docs/research/mc-26.2/05-worldgen.md`, with every moderate/low-confidence flag stated explicitly rather than presented as verified fact — restated as binding per this project's established discipline (M5-B07 Constraints (c), identical wording).

(d) **Gen-time block writes never call, or route through, `01`'s tick-time update engine** — every kind in this blueprint writes exclusively through `DecorationWorldAccess::set_block` (M5-B07 Constraints (d), unchanged, binding on this blueprint's new code too). No dependency edge from `rc-worldgen` to `rc-mechanics` is added.

(e) **No light-engine call of any kind** (M5-B07 Constraints (e), unchanged, binding here).

(f) **This blueprint's own confidence flags are never conflated with GEN-D20's one pinned exception** (Context §C restates M5-B07 Constraints (f)'s identical rule) — a code comment describing an approximated algorithm as "the GEN-D20 exception" is a documentation bug.

(g) **One small, explicitly-named gap is not resolved by editing `M5-B00-index.md`, `M5-B07-features-decoration.md`, `M5-B08-structures.md`, or any M5-B12a-e file** — all are outside this blueprint's assigned path. `chorus_plant` (Context §A/§B) is unimplemented anywhere in this project — harmless under GEN-D1's own End-out-of-scope framing (no corpus chunk exercises it), but named here rather than silently left inconsistent. (`blueprints/M5/M5-B00-index.md` owns the ID-reservation history and the full 64-kind coverage audit; it is not restated or re-litigated here.)

(h) **`driver.rs`'s per-feature call site is a three-blueprint sequence, not a single edit** (Context §D/§K.6) — this blueprint's own Deliverables text for that file is the correct final state, superseding M5-B12e's own intermediate one; an implementer who applies only M5-B07+the M5-B12 family (without this blueprint) has a completely valid, self-consistent intermediate state that simply lacks this blueprint's own 17 kinds, never a broken one.

(i) **No `unsafe` code.** Every function in this blueprint's Deliverables is implementable in safe Rust.

## Verification commands

- `cargo build -p rc-worldgen` — zero warnings.
- `cargo nextest run -p rc-worldgen` — every test in `vegetation_trunk_foliage.rs`, `vegetation_root_and_decorators.rs`, `vegetation_terminal_features.rs`, `vegetation_dispatch_composition.rs` passes.
- `cargo test --doc -p rc-worldgen` — exits 0.
- `cargo run -p xtask -- fmt-check` / `-- lint` / `-- lint-deps` — all exit 0.
- CI tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D37/D50).
