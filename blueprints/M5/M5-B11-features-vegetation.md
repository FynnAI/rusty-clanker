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

Close the **vegetation-classified** half of M5-B07's own named deferred backlog (Context §M/§N.5 of that blueprint): 17 terminal `Feature` kinds (trees, ocean/nether vegetation, mushrooms, vines, patches — Context §B's ledger), 6 of the 9-member `TrunkPlacer` family and 8 of the 11-member `FoliagePlacer` family (M5-B07 shipped 2 of each, this blueprint the other 6+8, leaving one named-deferred kind in each — `upwards_branching_trunk_placer` and `random_spread_foliage_placer`, Context §G/§H), the 1-member `RootPlacer` family (mangrove, unimplemented anywhere else), and 6 of the 10 `TreeDecorator` kinds — composed with the M5-B12 family's own already-shipped 35-kind non-vegetation half so that, together, this blueprint and the M5-B12 family reach every one of M5-B07's 57 named-deferred kinds (Context §B accounts for all 57: 17 here, 35 across M5-B12a-e, 5 End-exclusive and out of scope for both). This is the vegetation half of what the 10,000-chunk ≥99.9% hash-parity acceptance criterion (GEN-D27, `11-roadmap-milestones.md`) needs; the non-vegetation half is the M5-B12 family's own, already-drafted, and not re-derived here.

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
- `random.next_int_bounded(bound: i32) -> i32` — uniform over `[0, bound)`. `WorldgenRandom` overrides `next(bits)`, `fork`, `forkPositional` and `setSeed`, but never `next_int_bounded` itself, so `next_int_bounded` always runs the one backend-independent implementation (M5-B01's own documented quirk, unchanged): when `bound` is a power of two it returns the HIGH bits of a 31-bit draw, `((bound as i64) * next_bits(31) as i64) >> 31`, with no loop; otherwise it uses the classic rejection loop (`bits = next_bits(31); val = bits % bound; loop until (bits - val + bound - 1) >= 0 in wrapping 32-bit arithmetic`). Every `next_int_bounded(2)`/`next_int_bounded(4)` draw in this blueprint (corner-skip rolls, direction indices) takes the power-of-two fast path, not the rejection loop.
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
This blueprint's own `vegetation_patch`/`waterlogged_vegetation_patch` (Context §J, the only kind here that recurses) invokes `run_placement_chain` re-entrantly exactly as M5-B07's own `random_patch` does (M5-B07 Context §N.6: "recursively invoke `data.placed_features.get(&config.feature)`'s own full `run_placement_chain` at candidate as ITS new origin — a genuine nested full-placement-modifier-chain call") and exactly as M5-B12e's own selector/`sequence` kinds do (M5-B12e Context §K's `delegate` helper). **This blueprint restates one correction, matching what the reference actually does**: real vanilla's placement context carries an `Optional<PlacedFeature>` for the currently-placing top-level feature, not a feature id, and that optional is populated only at the TRUE top-level entry point; a nested re-entry (this blueprint's own `vegetation_patch`/`waterlogged_vegetation_patch` recursion included) passes an EMPTY optional, so a `Biome{}` modifier (M5-B07 Context §G.6) encountered inside a nested chain THROWS in real vanilla rather than checking any feature's own presence in the current biome's step list — real data packs simply never nest a `Biome{}` modifier this deep. This project's own `PlacementCtx::feature_name` stays the non-optional `&ResourceLocation` M5-B07 already established (unchanged); this blueprint's re-entrant call rebinds it to the nested feature's own id purely as this engine's own internal bookkeeping, never as a claim of matching vanilla's real `Biome{}`-inside-nested-chain behavior, which a `Biome{}` modifier is not expected to ever exercise in practice.

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
M5-B07 already implemented `TrunkPlacerJson::StraightTrunkPlacer`/`BendingTrunkPlacer` and `FoliagePlacerJson::BlobFoliagePlacer`/`SpruceFoliagePlacer` with a shared height formula (`height = base_height + random.next_int_bounded(height_rand_a + 1) + random.next_int_bounded(height_rand_b + 1)`, TWO draws, `height_rand_a` before `height_rand_b`, HIGH confidence) and `BlobFoliagePlacer`'s own algorithm (`radius` is sampled once by the tree driver itself, BEFORE the root and trunk placers run, not inside the foliage placer; `offset` is sampled once per `FoliageAttachment` inside the foliage placer — TWO draws overall, at two different call sites; per-layer SQUARE footprint, not a disk, with a probabilistic corner-skip that draws ONE `next_int_bounded(2)` per exact-corner candidate, taken before any `y == 0` special case). This blueprint's own `place_blob_foliage`/`place_spruce_foliage` helpers are assumed to already exist with those exact behaviors and are called directly by this blueprint's own `BushFoliagePlacer`/`JungleFoliagePlacer` (Context §H).

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

M5-B07 named 57 deferred `Feature` kinds (Context §B explains the "57 vs. 56" bookkeeping discrepancy in M5-B07's own summary prose) and 14 deferred trunk/foliage placer kinds, split along a vegetation/non-vegetation axis between this blueprint and the 5-blueprint M5-B12 family (M5-B12a — dripstone/geode/sculk, M5-B12b — nether geology/ice, M5-B12c — underground/structural misc, M5-B12d — fossil/template, M5-B12e — selectors/combinators and the family's own final combined dispatcher, `place_configured_feature_all`, Context §0). `blueprints/M5/M5-B00-index.md` owns the complete, authoritative 63-kind coverage table (6 by M5-B07 + 17 here + 35 across M5-B12a-e + 5 End-exclusive) and the ID-reservation history; this blueprint does not restate that audit, only consumes its result.

**This blueprint reconciles with the M5-B12 family's own real content.** Cross-checking the family's own vegetation-kind list (18 names: `fallen_tree`, `chorus_plant`, `huge_red_mushroom`, `huge_brown_mushroom`, `vines`, `vegetation_patch`, `waterlogged_vegetation_patch`, `seagrass`, `kelp`, `coral_tree`, `coral_mushroom`, `coral_claw`, `sea_pickle`, `bamboo`, `huge_fungus`, `nether_forest_vegetation`, `weeping_vines`, `twisting_vines`) against this blueprint's own independently-derived classification (Context §B) finds **agreement on 17 of those 18 names**; the one discrepancy is `chorus_plant`, listed elsewhere as an M5-B11-owned vegetation kind but genuinely End-dimension-exclusive in real vanilla (the Chorus Plant grows only in the End) — this blueprint does not implement it, for the identical dimension-scope reason M5-B07 §A already puts the whole End out of scope, restated in Context §I. The net effect: `chorus_plant` is unimplemented anywhere in this project — a genuine, small, honestly-flagged gap (Constraints (g)), not a silent omission, and harmless in practice since GEN-D1's own scope already excludes the End dimension entirely, so no real corpus chunk exercises it. Every kind the M5-B12 family implements as non-vegetation (its own 35 kinds — `no_op`, `random_selector`/`weighted_random_selector`/`simple_random_selector`/`random_boolean_selector`/`sequence`, `root_system`, `multiface_growth`, `block_pile`, `sculk_patch` among them) is left untouched by this blueprint, which defers entirely to the family's own already-shipped, self-consistent implementations rather than duplicating them.

**Composition, not modification.** The M5-B12 family explicitly declined to modify M5-B07's own `features/mod.rs`/`place_configured_feature` (M5-B12e's own Constraints (d): that file and function are "frozen"), instead defining its own new, combined dispatcher (`place_configured_feature_all`, Context §0) and having `driver.rs`'s single per-feature call site invoke that instead. This blueprint follows the identical discipline: it does not modify M5-B07's `features/mod.rs` either, and it does not modify the M5-B12 family's own `underground/mod.rs` (outside this blueprint's assigned path in any case). Instead, this blueprint defines its own dispatcher, `place_configured_feature_vegetation` (Context §D), which tries this blueprint's own 17 kinds first and falls through to M5-B12e's `place_configured_feature_all` for everything else — a pure composition, reaching every sibling blueprint's own kinds through one call chain, with `driver.rs`'s call site updated one further time to invoke this blueprint's own outer function (Context §D's own explicit ordering note).

**Dimension tiering** is unchanged from M5-B07's own Context §A: overworld and nether in scope, the End out of scope (Context §I).

### B. The complete 57-kind deferred-feature ledger, reconciled with the M5-B12 family

M5-B07 Context §M's own summary sentence says "56 remaining kinds" but its own enumerated list contains **57** distinct names (mechanically recounted from that blueprint's own text — the M5-B12 family's own derivation independently found the identical discrepancy and used the same resolution: the literal enumerated list, not the summary count, is authoritative). `random_patch`, one of M5-B07's own 7 originally-claimed implemented kinds, is **not a real vanilla feature kind at all**: it registers no `Feature` id, has no configuration type, and appears nowhere in the vanilla data pack; `docs/research/mc-26.2/05-worldgen.md` §3.13's own 63-name enumeration is correct as it stands, and M5-B07's own implemented-kind count is 6, not 7, once `random_patch` is removed — the true universe of confirmed vanilla feature kinds is **63**.

**Ownership, reconciled (Context §A):**

| Owner | Count | Kinds |
|---|---|---|
| M5-B07 (already implemented) | 6 | `ore`, `disk`, `spring_feature`, `lake`, `tree`, `simple_block` |
| **This blueprint (M5-B11) — 17** | 17 | `fallen_tree`, `huge_red_mushroom`, `huge_brown_mushroom`, `vines`, `vegetation_patch`, `waterlogged_vegetation_patch`, `seagrass`, `kelp`, `coral_tree`, `coral_mushroom`, `coral_claw`, `sea_pickle`, `bamboo`, `huge_fungus`, `nether_forest_vegetation`, `weeping_vines`, `twisting_vines` |
| M5-B12a-e (already implemented, not touched here) | 35 | M5-B12a: `large_dripstone`, `speleothem`, `speleothem_cluster`, `geode`, `sculk_patch`. M5-B12b: `delta_feature`, `basalt_columns`, `basalt_pillar`, `netherrack_replace_blobs`, `glowstone_blob`, `iceberg`, `blue_ice`, `freeze_top_layer`, `spike`. M5-B12c: `root_system`, `multiface_growth`, `underwater_magma`, `monster_room`, `block_pile`, `block_column`, `replace_single_block`, `block_blob`, `desert_well`, `void_start_platform`, `fill_layer`, `bonus_chest`. M5-B12d: `fossil`, `template`. M5-B12e: `no_op`, `random_selector`, `weighted_random_selector`, `simple_random_selector`, `random_boolean_selector`, `sequence`, `scattered_ore`. |
| Out of scope (End-exclusive) | 5 | `chorus_plant` (Context §A's own flagged gap), `end_platform`, `end_spike`, `end_island`, `end_gateway` |

`6 + 17 + 35 + 5 = 63` (the true total, matching the research corpus's own 63-name enumeration exactly; `random_patch` is not a real kind and is not counted). `17 + 35 + 5 = 57`, matching M5-B07's own literal deferred-list count exactly — every deferred kind is accounted for exactly once between this blueprint, the M5-B12 family, and the out-of-scope set. (M5-B07's own claim of a `random_patch` implementation is an M5-B07-owned fact outside this blueprint's edit scope — Constraints (g) — reported to the planning role rather than corrected here.)

**Trunk/foliage/root placer and tree-decorator ledger — entirely this blueprint's own, no split with the M5-B12 family exists for this family** (the M5-B12 family's own scope is strictly the `Feature` registry; `TrunkPlacer`/`FoliagePlacer`/`RootPlacer`/`TreeDecorator` are four separate registries it never touches): 6 of the vanilla registry's 9 trunk placers (`forking_trunk_placer`, `giant_trunk_placer`, `mega_jungle_trunk_placer`, `dark_oak_trunk_placer`, `fancy_trunk_placer`, `cherry_trunk_placer` — M5-B07 already shipped `straight_trunk_placer`/`bending_trunk_placer`, leaving `upwards_branching_trunk_placer` named-deferred, Context §G), 8 of the vanilla registry's 11 foliage placers (`pine_foliage_placer`, `acacia_foliage_placer`, `bush_foliage_placer`, `fancy_foliage_placer`, `jungle_foliage_placer`, `mega_pine_foliage_placer`, `dark_oak_foliage_placer`, `cherry_foliage_placer` — M5-B07 already shipped `blob_foliage_placer`/`spruce_foliage_placer`, leaving `random_spread_foliage_placer` named-deferred, Context §H), the 1-member `RootPlacer` family (`mangrove_root_placer`, Context §E), and 6 of the 10 `TreeDecorator` kinds (Context §F: `beehive`, `trunk_vine`, `leave_vine`, `cocoa`, `attached_to_leaves`, `attached_to_logs`; the remaining 4 — `pale_moss`, `creaking_heart`, `alter_ground`, `place_on_ground` — are named-deferred).

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

**Algorithm** (HIGH confidence — cross-checked against the reference). Invoked from `tree::place` (Implementation steps) BEFORE the trunk placer runs, only when `config.root_placer.is_some()`. There is no soil test on the block below `origin` in this algorithm at all — the mud/muddy-mangrove-roots requirement is a placement-MODIFIER gate on the placed feature (a `would_survive`-style predicate keyed on the tree's real support-block tag, broader than mud alone), applied before this root placer ever runs, not a check this root placer performs itself:

1. `trunk_offset = sample_int_provider(&config.trunk_offset_y, random)` (1+ draws, provider-dependent) — the real trunk placer's own `origin` is shifted UP by `trunk_offset` blocks from this root-placer's own `origin` (roots grow down from the tree's nominal base to the mud below; the visible trunk starts above that).
2. Walk the column from `origin` up to the shifted trunk origin, requiring every position to be air/replaceable-by-trees or a `can_grow_through`-tagged block (zero draws); if any position fails, the WHOLE tree placement is a no-op (mirrors vanilla's own "the shaft to the trunk is blocked" behavior).
3. Exactly FOUR root strands start, one per fixed horizontal direction in North, East, South, West order (Context §F's table) — never `max_root_width`-many, and with NO positional jitter draw; each strand's first candidate is `trunk_origin.relative(dir)`. `max_root_width` is not a strand count at all: it is a Manhattan-distance bound (from the root origin) tested against each candidate as the recursive walk proceeds. Per candidate, its distance `width` from the root origin decides the draw: if `width > max_root_width`, the candidate advances straight down with ZERO draws; otherwise ONE `skew_roll = random.next_float()` is drawn against `random_skew_chance` — in the outer band (`max_root_width - 3 < width <= max_root_width`) success yields BOTH `[below, sideways.below()]` while failure yields `[below]` with no further draw; in the inner band (`width <= max_root_width - 3`) success yields `[below]` while failure draws a SECOND `random.next_bool()` (ONE draw) choosing between `[sideways]` and `[below]` — "sideways" is always the strand's own already-established direction, never a freshly drawn `next_int_bounded(4)`. Growth is a depth-first recursion over these candidates (not a straight downward walk), placing `sample_block_state_provider(&config.root_provider, random, resolver)` at each visited position; a candidate failing the air/replaceable/`can_grow_through` test is simply skipped (the strand does NOT stop early on it) and the recursion continues to the next candidate — the only real early exit is a layer/length guard (`layer == max_root_length` or the accumulated position count exceeds `max_root_length`), which returns failure and aborts the WHOLE tree, not just that strand.

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
    LeaveVine { probability: f32 },
    Cocoa { probability: f32 },
    AttachedToLeaves { probability: f32, exclusion_radius_xz: i32, exclusion_radius_y: i32, required_empty_blocks: i32, block_provider: crate::decoration::providers::BlockStateProvider, directions: Vec<String> },
    AttachedToLogs { probability: f32, block_provider: crate::decoration::providers::BlockStateProvider, directions: Vec<String> },
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

**`beehive`** (`Beehive{probability}`, HIGH confidence — cross-checked against the reference). `roll = random.next_float()` (ONE draw); if `roll >= probability`, stop — zero further draws (this gate is real; nothing resembling a single direction-index draw follows it). On success: `log.log_positions`/`log.leaf_positions` are treated ascending by Y, so the FIRST (lowest) entries drive `hive_y`: if leaves are non-empty, `hive_y = max(lowest_leaf.y - 1, lowest_log.y + 1)` (zero draws); if there are no leaves, draw `y_roll = random.next_int_bounded(3)` (ONE draw) and set `hive_y = min(lowest_log.y + 1 + y_roll, highest_log.y)`. Build the candidate list: every log at `hive_y`, offset ONE step (not two) in each of three fixed directions — East, South, West (the horizontal plane minus North) — never all four. Shuffle that candidate list with a Fisher-Yates shuffle (one `next_int_bounded(i)` draw per element, for `i` from the candidate count down to 2). Take the FIRST surviving candidate for which `world.get_block(candidate)` is STRICTLY air (not air-or-replaceable) AND `world.get_block(candidate + (0,0,1))` (one step SOUTH — the hive's fixed facing direction, not a leaf test and not the chosen offset direction) is also strictly air. On a successful placement: `resolver.resolve(&BlockStateSpec{block: "minecraft:bee_nest", properties: {"facing": "south"}})` (facing is always South, never the offset direction), then draw `bee_count = 2 + random.next_int_bounded(2)` (ONE draw) and, per bee, `random.next_int_bounded(599)` (one draw each) for its own occupant data. This blueprint places the bare block state only and skips the occupant-count/per-bee draws — a documented, bounded simplification (a bare-blockstate implementation therefore desyncs every later feature's own RNG stream relative to real vanilla, flagged here rather than silently accepted) — the bee-nest block-entity's own occupant/honey-level NBT payload is a `05-game-mechanics.md` concern; honey level itself is a block-state property of `bee_nest`, not block-entity NBT, and this blueprint's placed state never sets it either, out of this blueprint's own scope.

**`trunk_vine`** (`TrunkVine{}`, HIGH confidence — cross-checked against the reference). For `pos` in `log.log_positions` (declared accumulation order), for `dir_idx` in the FIXED order West, East, North, South (not N,E,S,W): draw `roll = random.next_int_bounded(3)` (ONE draw PER direction PER log, UNCONDITIONAL — drawn before the air test, not gated by it); place a vine iff `roll > 0` (probability 2/3, not `roll == 0`) AND `world.get_block(pos + dir_offset(dir_idx))` is air; the vine's own facing property is the OPPOSITE face of the offset direction (the West neighbor gets a vine facing East, the East neighbor facing West, and so on) — the vine hangs off the log toward the log, not toward the empty neighbor.

**`leave_vine`** (`LeaveVine{probability}`, HIGH confidence — cross-checked against the reference; shares only its four unrolled West/East/North/South direction blocks with `trunk_vine`). For `pos` in `log.leaf_positions`, for `dir_idx` in the fixed West, East, North, South order: draw `roll = random.next_float()` (ONE draw PER direction PER leaf, against the codec-configured `probability`, NOT `trunk_vine`'s `next_int_bounded(3)`); on success (`roll < probability`), place a vine on that face and CONTINUE it downward: extend through up to 4 further positions below, one at a time, stopping the first time a position is not air (each extension attempt draws further, matching this project's own established `FoliagePlacer`-style hanging-vine extension convention) — an extra downward-growth behavior `trunk_vine` has no counterpart for.

**`cocoa`** (`Cocoa{probability}`, HIGH confidence — cross-checked against the reference). `roll = random.next_float()` (ONE draw for the WHOLE decorator, taken once before any log is visited — NOT once per log). If `roll >= probability`, stop with zero further draws. Otherwise, restrict to logs within 2 of the lowest log's Y (zero draws); for each surviving log, for EACH of the four fixed North/East/South/West directions: draw `face_roll = random.next_float()` (ONE draw PER direction PER log, unconditional) against a FIXED `0.25` (not the configured `probability`, which only gates the whole decorator once); on success (`face_roll <= 0.25`), the pod position is the log offset in the OPPOSITE of that direction, and its age is `random.next_int_bounded(3)` (ONE draw, NOT a fixed `0`); place `cocoa[facing=dir,age=age]` there iff it is air.

**`attached_to_leaves`** (moderate-low confidence, cross-checked draw order). The leaf list is shuffled FIRST (`Util.shuffledCopy`-equivalent, one `next_int_bounded(i)` draw per element, `i` from the leaf count down to 2); per surviving leaf, in shuffled order: draw `dir_idx = random.next_int_bounded(config.directions.len() as i32)` (ONE draw, UNCONDITIONAL and FIRST — `config.directions` has no all-four-cardinals default, it is a required non-empty list); only THEN draw `roll = random.next_float()` (ONE draw, gated by a blacklist-miss check that short-circuits it away entirely for a position already inside an exclusion box); on success, walk up to `required_empty_blocks` positions in `dir_idx`'s direction checking each is air (zero draws), and if all are, place `sample_block_state_provider(&block_provider, random, resolver)` at the first one, then grow a blacklist box of `exclusion_radius_xz`/`exclusion_radius_y` around THAT placed position (not around the tree's own origin) so later leaves in the same pass avoid it.

**`attached_to_logs`** (moderate-low confidence, cross-checked field set). This kind has NO `exclusion_radius_xz`/`exclusion_radius_y`/`required_empty_blocks` fields at all. The log list is shuffled first (same shuffle discipline as `attached_to_leaves`); per surviving log, in shuffled order: draw `dir_idx = random.next_int_bounded(config.directions.len() as i32)` (ONE draw, unconditional); draw `roll = random.next_float()` (ONE draw); if `roll < probability` AND `world.get_block(pos + dir_offset(dir_idx))` is air, place `sample_block_state_provider(&block_provider, random, resolver)` there — no exclusion-radius bookkeeping and no multi-step empty-block walk.

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

The vanilla `TrunkPlacer` registry has **nine** kinds, not eight: `straight_trunk_placer`, `forking_trunk_placer`, `giant_trunk_placer`, `mega_jungle_trunk_placer`, `dark_oak_trunk_placer`, `fancy_trunk_placer`, `bending_trunk_placer`, `upwards_branching_trunk_placer` and `cherry_trunk_placer`. `TrunkPlacerJson` gains 6 new variants (M5-B07's already-shipped `StraightTrunkPlacer`/`BendingTrunkPlacer`/`Unsupported` arms are unchanged); the 9th kind, `upwards_branching_trunk_placer` — the one both mangrove configured features actually use — remains named-deferred, an honestly-flagged gap (not this blueprint's `RootPlacer`, Context §E, which is a different mechanism): `Unsupported` still catches it, alongside any genuinely unrecognized future kind:

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

**`forking_trunk_placer`** (HIGH confidence — matches the reference exactly). This is a leaning main trunk with one conditional fork, not a straight column plus two symmetric branches. Height draw (2, shared formula). `lean_dir = random.next_int_bounded(4)` (ONE draw); `lean_height = height - random.next_int_bounded(4) - 1` (ONE draw); `lean_steps = 3 - random.next_int_bounded(3)` (ONE draw). Place a straight single-log column from `origin.y` up to `lean_height` layers, then drift one block per layer in `lean_dir` for `lean_steps` further layers (fresh `trunk_provider` sample per log); this main column DOES emit one `FoliageAttachment` at its own top (`radius_offset: 1, double_trunk: false`). A second `next_dir = random.next_int_bounded(4)` (ONE draw) is drawn UNCONDITIONALLY; only if `next_dir != lean_dir` does a fork occur: `branch_pos = lean_height - random.next_int_bounded(2) - 1` (ONE draw) and `branch_len = 1 + random.next_int_bounded(3)` (ONE draw, 1 to 3 logs) are drawn, and the branch steps diagonally outward (`+dir_offset`) and upward (`+1` Y) from `branch_pos`, adding a second `FoliageAttachment` (`radius_offset: 0, double_trunk: false`) at its own top. TOTAL draws: `2 + 3 = 5` when the fork does not occur, `2 + 5 = 7` when it does; `FoliageAttachment` count is 1 or 2 accordingly.

**`giant_trunk_placer`** (HIGH confidence — this blueprint's designated exact hand-trace, Acceptance tests). Height draw (2, shared formula). First places four below-trunk blocks at `origin.below()` and its east/south/south-east neighbours (fresh sample per column, zero draws for a non-drawing provider). Per layer `y` in `origin.y ..= origin.y + height - 1`: the `(x,y,z)` column is stamped on EVERY layer, while the `(x+1,y,z)`, `(x+1,y,z+1)` and `(x,y,z+1)` columns are stamped only while `y < origin.y + height - 1` — so the TOP layer is a single column, not a full 2×2 (the 2×2 footprint holds for every layer below the top). Zero further draws. Returns ONE `FoliageAttachment { pos: (origin.x, origin.y + height, origin.z), radius_offset: 0, double_trunk: true }`.

**`mega_jungle_trunk_placer`** (HIGH confidence — matches the reference exactly). Calls `giant_trunk_placer`'s own routine first (the identical below-trunk writes, 2×2-minus-top-layer stamp, and its one `FoliageAttachment`), THEN runs an additional randomized branch loop the giant placer does not have: `branch_height` starts at `height - 2 - random.next_int_bounded(4)` (ONE draw) and the loop continues while `branch_height > height / 2`, decrementing `branch_height` by `2 + random.next_int_bounded(4)` (ONE draw per iteration) each pass. Per iteration: `angle = random.next_float() * 2π` (ONE draw); a 5-block diagonal branch is stamped stepping outward by `cos(angle)`/`sin(angle)` and rising `0.5` Y per step from `(origin.x, origin.y + branch_height, origin.z)`; the branch adds one further `FoliageAttachment` (`radius_offset: -2, double_trunk: false`). TOTAL draws: giant's own `2` (height) plus `1` (the loop's starting draw) plus `2` per loop iteration (the decrement draw plus the angle draw) — genuinely variable, since the iteration count depends on `height` and the drawn decrements.

**`dark_oak_trunk_placer`** (HIGH confidence — matches the reference exactly). Height draw (2, shared formula). Places four below-trunk blocks (as `giant_trunk_placer`). `lean_dir = random.next_int_bounded(4)` (ONE draw); `lean_height = height - random.next_int_bounded(4)` (ONE draw); `lean_steps = 2 - random.next_int_bounded(3)` (ONE draw). Per layer, the 2×2 footprint is stamped only where the base position is air-or-leaves (drifting by `lean_dir` past `lean_height` for `lean_steps` layers, same drift shape as `forking_trunk_placer`'s lean). Returns ONE `FoliageAttachment` at `(trunk_x, origin.y + height - 1, trunk_z)` — the TOP layer itself, not `y + 1` — with `double_trunk: true`. THEN, for `ox, oz` each ranging `-1..=2` (a 4×4 ring around the inner 2×2, 12 outer cells): each cell draws `random.next_int_bounded(3)` (ONE draw per cell); when the result is `<= 0`, a further `branch_len = 2 + random.next_int_bounded(3)` (ONE draw) is drawn, a downward log column of that length is stamped from one below the top layer, and a further `FoliageAttachment` at `(x+ox, top_y, z+oz)` with `double_trunk: false` is added. TOTAL attachments: 1 to 13 (the base one plus 0 to 12 ring branches); TOTAL draws: `2 + 3 + 12 + (0..12)`, genuinely data/RNG-dependent.

**`fancy_trunk_placer`** (LOW confidence on the branch walk itself, HIGH confidence on the scan/height formulas below — this blueprint's own bounded simplification of vanilla's real, genuinely complex "big oak" branch-slope algorithm, named as such rather than silently passed off as exact, matching M5-B07's own `lake` feature precedent). Height draw (2, shared formula). `crown_height = height + 2` (deterministic, zero draws); `trunk_height = (crown_height as f32 * 0.618).floor() as i32` (golden-ratio split point on `crown_height`, not the raw drawn `height`; zero draws, deterministic). There is no drawn branch count, direction index, or branch length anywhere in this placer: branches instead come from a descending scan of `relative_y` from `crown_height - 5` down to `0` (skipping any layer whose shape factor is non-positive); at each eligible layer, up to `clusters_per_y = min(1, floor(1.382 + (crown_height / 13.0)^2))` cluster attempts run, and EACH attempt draws `radius = shape_factor * (random.next_float() + 0.328)` (ONE draw) then `angle = random.next_float() * 2π` (ONE draw), turned into an x/z branch-start offset by `sin`/`cos` — TWO draws per cluster attempt, not a direction index or length. Each branch's own base Y is `checkpoint.y - sqrt(dx² + dz²) * 0.381`, clamped down to `trunk_height`'s own top (deterministic, zero draws); branch length is never sampled — it falls out of the branch's own walk toward its computed endpoint. Only branches whose base Y is at least `crown_height * 0.2` are kept. The straight single-log trunk column (`origin.y` to `origin.y + trunk_height - 1`) is placed only AFTER this whole scan completes. Each retained branch's endpoint is one `FoliageAttachment`. TOTAL draws: `2 + 2` per cluster attempt across every eligible descending layer — genuinely data/RNG-dependent, not a fixed `2 + 1 + 2*branch_count`.

**`cherry_trunk_placer`** (moderate confidence on the extra field set — public datapack schema convention; HIGH confidence on the draw order and branch shape below, cross-checked against the reference). Height draw (2, shared formula). Cherry has exactly TWO branches, not `branch_count`-many: `first_start_offset = sample_int_provider(&config.branch_start_offset_from_top, random)` (1+ draws) and `second_start_offset` (from a derived, narrower `IntProvider`, 1+ draws) are BOTH sampled first, before anything else, with a collision bump between them if needed (zero further draws). `branch_count_val = sample_int_provider(&config.branch_count, random)` (1+ draws) is sampled next, but only feeds `trunk_height`'s own derivation (`trunk_height` equals the drawn `height` only for a three-branch tree; otherwise it is shorter) — it does NOT control how many branches are placed. A straight single-log column is placed to `trunk_height`. Exactly ONE `tree_direction = random.next_int_bounded(4)` (ONE draw) is drawn for the WHOLE placer; the second branch uses its opposite direction, never a fresh draw. Each of the two branches then independently draws `branch_end_offset = sample_int_provider(&config.branch_end_offset_from_top, random)` (1+ draws) then `horiz_len = sample_int_provider(&config.branch_horizontal_length, random)` (1+ draws); the branch takes 1 or 2 fixed horizontal steps first, then walks toward its computed end position, drawing `random.next_float()` against `|ΔY| / manhattan_distance` at EVERY remaining step to choose a vertical vs. a horizontal step (ONE draw per step) — a gentle drooping staircase, not a flat-Y horizontal run; `branch_end_offset_from_top` steers this whole staircase's target, not merely a final nudge. Each branch's end is one `FoliageAttachment`; TOTAL `FoliageAttachment` count is always exactly 2.

### H. Full `FoliagePlacer` family — the 8 new kinds, and the multi-attachment driving loop

**Driving loop** (this blueprint's own restatement of vanilla's real "call once per attachment, sharing one continuing RNG stream" behavior): `tree::place` (Implementation steps) calls `place_foliage(&config.foliage_placer, &config.foliage_provider, &attachment, world, resolver, random)` once per `FoliageAttachment` the trunk placer returned, in the SAME order the trunk placer produced them — every attachment's own draws happen fully before the next attachment's own call begins (same depth-first discipline as `run_placement_chain`, Context §0).

```rust
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FoliagePlacerJson {
    BlobFoliagePlacer { radius: crate::data::IntProvider, offset: crate::data::IntProvider, height: i32 },
    SpruceFoliagePlacer { radius: crate::data::IntProvider, offset: crate::data::IntProvider, trunk_height: crate::data::IntProvider },
    PineFoliagePlacer { radius: crate::data::IntProvider, offset: crate::data::IntProvider, height: crate::data::IntProvider },
    AcaciaFoliagePlacer { radius: crate::data::IntProvider, offset: crate::data::IntProvider },
    BushFoliagePlacer { radius: crate::data::IntProvider, offset: crate::data::IntProvider, height: i32 },
    FancyFoliagePlacer { radius: crate::data::IntProvider, offset: crate::data::IntProvider, height: i32 },
    JungleFoliagePlacer { radius: crate::data::IntProvider, offset: crate::data::IntProvider, height: i32 },
    MegaPineFoliagePlacer { radius: crate::data::IntProvider, offset: crate::data::IntProvider, crown_height: crate::data::IntProvider },
    DarkOakFoliagePlacer { radius: crate::data::IntProvider, offset: crate::data::IntProvider },
    CherryFoliagePlacer { radius: crate::data::IntProvider, offset: crate::data::IntProvider, height: crate::data::IntProvider },
    #[serde(other)]
    Unsupported,
}
```

**`pine_foliage_placer`** (HIGH confidence — cross-checked against the reference). `radius0 = sample_int_provider(&radius, random)` PLUS a second draw, `radius0 += random.next_int_bounded(max(trunk_height+1, 1))` (so the radius is TWO draws by itself, not one), `offset0 = sample_int_provider(&offset, random)` (a third draw), and `foliage_height_val = sample_int_provider(&height_provider, random)` (a fourth draw — this placer's own `height` field is itself sampled, not a fixed integer). TOTAL draws: 4, not 2. For each descending layer: `current_radius` starts at 0 and INCREMENTS while below `radius0` (with `radius_offset`), decrementing once back down near the top of the descent (a ramp-up-then-ramp-down profile, not a fixed even/odd alternation); fill a SQUARE footprint (this part of the original description holds) where the target is air-or-replaceable, fresh `foliage_provider` sample per block; the per-corner skip test drops the exact corner whenever `current_radius > 0`, deterministically, with NO draw (unlike Blob's own probabilistic corner-skip).

**`acacia_foliage_placer`** (HIGH confidence — cross-checked against the reference). `radius0`, `offset0` (2 draws total — its own `height` field returns a fixed `0`, so no third draw). THREE `placeLeavesRow`-style layers, not one: at `y = -1 - 0` with radius `radius0 + radius_offset`, at `y = -0` with radius `radius0 - 1`, and at `y = 0` with radius `radius0 + radius_offset - 1`. The skip test is layer-dependent: on the `y == 0` layer it drops any position with `(|dx| > 1 or |dz| > 1) and dx != 0 and dz != 0` — carving a plus/cross shape, not four corners; on every other layer it drops the exact corner whenever `current_radius > 0`, deterministically. TOTAL draws: 2, both structurally correct, but the geometry is a 3-row cross-carved shape, not a single flat disk with corners removed.

**`bush_foliage_placer`** (HIGH confidence — cross-checked against the reference; shares only its DRAW shape with Blob, not its geometry). Same 2 provider draws plus one `next_int_bounded(2)` per exact-corner candidate as Blob, but the per-layer radius is `radius0 + radius_offset - 1 - yo` (no halving of `yo`, and no clamp at zero — Blob's own formula is `max(radius0 + radius_offset - 1 - yo/2, 0)`), and the skip test is the exact-corner-with-`next_int_bounded(2)` test ALONE, dropping Blob's additional `or y == 0` clause — so, in a standard config with a descending `yo`, Bush keeps the layer at `y == 0`'s own corners half the time where Blob always removes them (Blob's own `next_int_bounded(2)` draw is still taken either way).

**`jungle_foliage_placer`** (HIGH confidence — cross-checked against the reference; not Blob with one overridden layer). Draws an EXTRA value beyond radius/offset when the attachment is NOT double-trunked: `leaf_height = if double_trunk { foliage_height_const } else { 1 + random.next_int_bounded(2) }` (a third draw in the single-trunk case). Per-layer radius is `radius0 + radius_offset + 1 - yo` (not Blob's own halved/clamped formula). The skip test is a true CIRCLE, not a corner roll: skip when `|dx| + |dz| >= 7`, else skip when `dx*dx + dz*dz > current_radius*current_radius` — zero draws in the skip test at all, no `next_int_bounded(2)` corner roll.

**`fancy_foliage_placer`** (HIGH confidence — cross-checked against the reference). `radius0`, `offset0` (2 draws; its own `height` field is a fixed constant, inherited from Blob, so no third draw). For each layer descending from `offset0` to `offset0 - foliage_height`: `current_radius = radius0` on the FIRST and LAST layers, `radius0 + 1` in between (NOT `radius0 - abs(layer - height/2)/2`, and it never adds `radius_offset`); the skip test keeps positions where `(dx+0.5)^2 + (dz+0.5)^2 <= current_radius^2` (a half-offset circle, NOT `dx*dx+dz*dz <= layer_radius*layer_radius + 1`).

**`mega_pine_foliage_placer`** (HIGH confidence — cross-checked against the reference). `radius0`, `offset0`, `crown = sample_int_provider(&crown_height, random)` (3 draws — this part holds). There is NO constant-radius crown band and NO even/odd alternation: for `yo = attachment.pos.y - yy` at each ascending layer, `smooth_radius = radius0 + radius_offset + floor(yo / crown as f32 * 3.5)` (radius GROWS toward the bottom of the crown), bumped by 1 when `yo > 0` and the radius repeats an even `yy`. The skip test is the same circular test as `jungle_foliage_placer` (`|dx|+|dz| >= 7`, else `dx*dx+dz*dz > current_radius*current_radius`) — no draws, no corner roll.

**`dark_oak_foliage_placer`** (HIGH confidence — cross-checked against the reference). The radius provider is sampled exactly ONCE PER TREE by the tree driver itself (Context §G's shared-radius note), never per attachment; only `offset0` is sampled per `FoliageAttachment` call (ONE draw per call, not two), and its own `height` field is a fixed constant `4`, zero draws. `DarkOakTrunkPlacer` returns 1 to 13 attachments (Context §G), not two. For a `double_trunk: true` attachment: three rows at `y = -1, 0, 1` with radii `radius0+2, radius0+3, radius0+2`, plus a `random.next_bool()` draw (ONE draw) that may add a FOURTH row at `y = 2` with radius `radius0`. For every other attachment: two rows, `radius0+2` at `y = -1` and `radius0+1` at `y = 0`. Corner-skipping stays deterministic (no draw), as originally stated.

**`cherry_foliage_placer`** (HIGH confidence — cross-checked against the reference; uses NO integer rolls at all, unlike Blob). `radius0`, `offset0`, and `foliage_height_val = sample_int_provider(&height_provider, random)` (THREE draws — its own `height` field is itself sampled, not a constant). `current_radius = radius0 + radius_offset - 1`. Its own layer stack is NOT Blob's disk shape: two tapered rows near the top (`current_radius - 2` at `y = foliage_height - 3`, `current_radius - 1` at `y = foliage_height - 4`), a run of full-radius rows from `y = foliage_height - 5` down to `0`, then two extending "hanging-leaves-below" rows at `y = -1` and `y = -2` whose own extension attempts draw further floats. The skip test uses ONLY floats, never `next_int_bounded`: on the `y == -1` edge, an edge position draws `random.next_float()` against a configured `wide_bottom_layer_hole_chance`; on a wide layer (`current_radius > 2`) an exact corner is dropped UNCONDITIONALLY with no draw, while the near-corner band (`|dx|+|dz| > current_radius*2-2`) draws `random.next_float()` against a configured `corner_hole_chance`; only on a NARROW layer (`current_radius <= 2`) does the exact corner itself draw against `corner_hole_chance`. Every interior position takes no draw at all.

### I. Out-of-scope kinds, restated (End-exclusive, 5 kinds)

`chorus_plant`, `end_platform`, `end_spike`, `end_island`, `end_gateway` are NOT dispatched by `place_configured_feature_vegetation` — a `ConfiguredFeature` naming any of these five `feature_type` strings falls all the way through to `place_configured_feature_all`'s own "unrecognized kind, documented `debug`-logged no-op" default (Context §0), identical in observable behavior to every other not-yet-implemented kind, but for a structurally different reason (dimension-scope exclusion, not an implementation gap) — restated here so a future reader never mistakes "the End is out of scope" for "nobody got around to it yet." `void_start_platform`, despite its similar End-adjacent name, is **not** in this list — M5-B12c already implements it (its own Context §L.2), since it turns out to be usable outside the End too. Revisit only if a future milestone brings the End dimension into GEN-D1's scope.

### J. Config structs and algorithms — every terminal kind this blueprint owns (17)

Every kind below is a `Configuration` struct plus one `pub fn place(config: &..Configuration, origin: rc_core::BlockPos, world: &mut dyn DecorationWorldAccess, resolver: &dyn BlockStateResolver, props: &dyn BlockPropertyResolver, random: &mut WorldgenRandom<AnyRandom>)`, matching M5-B07's/M5-B12's own established per-kind file convention exactly (Deliverables lists the concrete file placement). The fixed direction table from Context §F applies identically here.

**`fallen_tree`** (HIGH confidence — cross-checked against the reference). Places a stump log at `origin` UNCONDITIONALLY first (before any draw) and runs any configured stump decorators. Then: `dir_idx = random.next_int_bounded(4)` (ONE draw); `len = sample_int_provider(&config.log_length, random) - 2` (1+ draws — note the MINUS 2); the log's own start position is `origin.relative(dir_idx, 2 + random.next_int_bounded(2))` (ONE further draw) — 2 or 3 blocks out from `origin`, never at `origin` itself. Ground height is found by probing up 1 and then down up to 6 further positions (zero draws); placement is accepted as long as at most 2 CONSECUTIVE non-solid support columns occur along the log's own length (not a single "solid block below origin" gate); place `len` consecutive `trunk_provider` samples (fresh sample per log, correct horizontal log-axis property implied by `dir_idx`'s axis) from the found start position.

**`huge_fungus`** (moderate confidence on the hat's exact decor/fill thresholds below, HIGH confidence on the gate/height/stem facts — cross-checked against the reference). Gate: the block below `origin` must ALWAYS match `config.valid_base_state` — `config.planted` does NOT skip this test; `planted` only skips a separate generation-depth ceiling check this blueprint does not model. `total_height = 4 + random.next_int_bounded(9)` (ONE draw, 4 to 12 inclusive — not `5 + next_int_bounded(3)`), then DOUBLED when `random.next_int_bounded(12) == 0` (a SECOND draw). `is_huge = !config.planted && random.next_float() < 0.06` (a THIRD draw); when true the stem widens to a 3×3 footprint, and each of its 4 corner columns additionally draws `random.next_float() < 0.1` (one draw per corner per layer) to decide whether that corner is filled. Straight `stem_state` column for `total_height` layers (zero further draws for a fixed `stem_state`). The hat is NOT a fixed-radius-3 disk: `hat_height = min(5 + random.next_int_bounded(1 + total_height / 3), total_height)` (ONE draw); per hat layer, radius is `2` while `dy < total_height - random.next_int_bounded(3)` (ONE draw per layer) else `1`, forced to `3` near the top when `hat_height > 8`, and incremented by 1 when `is_huge`. Per hat-candidate block: draw `random.next_float()` against a position-dependent decor chance (roughly `0.1` interior, `0.01` at corners, `5e-4` at edges — this blueprint's own best-effort match to those three bands) and then a SECOND `random.next_float()` against a hat-fill chance — never a single `next_int_bounded(5) == 0` roll.

**`huge_red_mushroom` / `huge_brown_mushroom`** (SHARED `HugeMushroomConfiguration { cap_provider: BlockStateProvider, stem_provider: BlockStateProvider, foliage_radius: i32 }`, one Rust function `place_huge_mushroom(is_red: bool, ..)` dispatched from both `feature_type` strings — HIGH confidence on height/stem below, LOW-MODERATE on the cap geometry, restated explicitly as an approximation rather than a byte-exact claim). `height = 4 + random.next_int_bounded(3)` (ONE draw, 4 to 6), then DOUBLED when `random.next_int_bounded(12) == 0` (a SECOND draw) — giving 4-6 OR 8-12, never a flat 4-6. The cap is placed BEFORE the stem. Straight `stem_provider` column for the FULL `height` layers (not `height - 1`; the cap's own lower layers, below, overlap the stem's own top layers). Cap, deterministic given height (zero further RNG for a non-drawing `cap_provider`; a weighted `cap_provider` would draw once per candidate position): for `is_red`, FOUR Y-layers, `height-3 ..= height` inclusive (no clamp at 0), radius `config.foliage_radius` on the three LOWER layers and `foliage_radius - 1` on the TOP layer only (the top layer is NARROWER, not wider); the fill test keeps the TOP layer solid everywhere, while each of the three lower layers keeps only positions where EXACTLY ONE of the X/Z edge flags is set — an edge ring with its own corners already excluded, not a solid disk. For `is_brown`, a SINGLE layer at `dy == height`, radius `config.foliage_radius` (the SAME radius as red's lower layers, not one larger), filled everywhere EXCEPT where BOTH the X and Z edge flags are set (i.e. the four corners ARE notched for brown, the opposite of red's own top-layer/lower-layer split) — giving red its rounded, hollow-ring-then-solid-cap silhouette and brown its flat, corner-notched table-top.

**`vegetation_patch` / `waterlogged_vegetation_patch`** (SHARED `VegetationPatchConfiguration { ground_state: BlockStateProvider, vegetation_feature: crate::data::ResourceLocation, replaceable_tag: String, depth: crate::data::IntProvider, #[serde(default)] extra_bottom_block_chance: f32, vertical_range: i32, xz_radius: crate::data::IntProvider, #[serde(default)] extra_edge_column_chance: f32 }`, one function `place_vegetation_patch(waterlogged: bool, ..)` dispatched from both `feature_type` strings — this is the ONLY kind in this blueprint that needs the extended `(data, ctx)` parameters, since it recurses into `vegetation_feature` (Context §0) — HIGH confidence, cross-checked against the reference). `x_radius = sample_int_provider(&config.xz_radius, random) + 1` and, as an INDEPENDENT second draw, `z_radius = sample_int_provider(&config.xz_radius, random) + 1` (TWO separate draws, X before Z, each one more than the provider's own sampled value — not one shared radius). For `dx in -x_radius..=x_radius, dz in -z_radius..=z_radius` (ascending `dx` then `dz`): the edge test is RECTANGULAR, not circular — `is_x_edge = dx == -x_radius or dx == x_radius`, `is_z_edge` likewise; a CORNER (both edges) is skipped UNCONDITIONALLY with zero draws; an edge-but-not-corner column draws `edge_roll = random.next_float()` (ONE draw) ONLY when `config.extra_edge_column_chance != 0.0` (short-circuited away entirely at chance `0`, so not always one draw per boundary column) and is skipped unless the roll is at most that chance; an interior column draws nothing. For each surviving column: move INWARD (along the configured surface direction) while the position is air, up to `vertical_range` steps, then OUTWARD while non-air, up to `vertical_range` further steps (zero draws); accept the position when it is empty AND the block one step further inward has a sturdy face toward it — the `replaceable_tag` is consulted only later, while writing ground blocks, never during this scan. The `waterlogged` variant does not require pre-existing water either: it instead keeps surface positions that are NOT exposed on North/East/South/West/Down and then SETS water into them. `depth = sample_int_provider(&config.depth, random)` (1+ draws) PLUS one more when `config.extra_bottom_block_chance > 0.0` and a further `random.next_float()` (ONE draw, short-circuited away at chance `0`) is below that chance; ground blocks are then written starting one step further INWARD of the accepted air position (never at the surface position itself), stepping further inward for `depth` blocks total.

**`vines`** (`VineConfiguration {}`, zero fields, HIGH confidence — cross-checked against the reference; the ordinary overworld climbing vine, not to be confused with M5-B12's own, entirely separate `multiface_growth`/glow-lichen kind). Consumes ZERO RNG draws and touches exactly ONE position: if `world.get_block(origin)` is not empty, stop. Otherwise, iterate the six directions in the FIXED order Down, Up, North, South, West, East, skipping Down; on the FIRST direction whose neighbor is an acceptable vine-support block, set a single vine face in that direction and stop — never all four cardinal directions, never a 9×9 footprint, and no `next_bool()` roll anywhere.

**`weeping_vines` / `twisting_vines`** (moderate confidence on the exact scatter/height constants below — a bounded reconstruction of vanilla's real multi-hundred-attempt scatter algorithm, HIGH confidence on the two corrected facts: `weeping_vines` is genuinely config-free (`NoneFeatureConfiguration`), but `twisting_vines` is NOT — it carries its own `TwistingVinesConfig { spread_width: i32, spread_height: i32, max_height: i32 }`, read at placement time, so only `weeping_vines` is "hardcoded, non-datapack-configurable"). Neither feature is a single column at `origin`: `weeping_vines` requires `origin` to be empty with netherrack or a nether-wart-adjacent block above, places a wart block at `origin`, then runs 200 scatter attempts (each offsetting from `origin` by a difference-of-two-draws per axis) to spread wart blocks, then a further 100 scatter attempts placing the actual vines — each successful hit draws its own `vine_height` (roughly `1` to `8`, occasionally doubled or forced to `1` by extra rolls) and places that many "plant" body segments plus one distinct "head" block whose age is drawn separately. `twisting_vines` runs `config.spread_width * config.spread_width` scatter attempts (offsets bounded by `spread_width`/`spread_height`, height rolls bounded by `config.max_height`) with the same plant-segments-plus-head shape. This blueprint's own simpler "one column at `origin`, height = 8 + next_int_bounded(4)" implementation remains a NAMED, bounded simplification of this real scatter behavior, not a byte-exact restatement.

**`bamboo`** (`BambooConfiguration { probability: f32 }`, HIGH confidence — cross-checked against the reference). `stalk_height = 5 + random.next_int_bounded(12)` (ONE draw, 5–16) is drawn FIRST and UNCONDITIONALLY — the probability roll comes AFTER and gates only a separate podzol disk, not the bamboo growth itself: `roll = random.next_float()`; if `roll < config.probability`, a podzol-disk radius `r = 1 + random.next_int_bounded(4)` (ONE further draw) is sampled and a circular podzol patch is placed at the surface height below. The stalk column places up to `stalk_height` plain, leafless bamboo-trunk blocks while the position stays empty (stopping early otherwise); leaves are NOT thirds: only if the column reached at least 3 blocks does it get a leafed top — one NEW block placed one above the last trunk block (`leaves=large`, the topmost), and the two already-placed trunk blocks just below it REWRITTEN in place (`leaves=large` then `leaves=small` descending) — so the finished stalk is one block taller than the trunk loop itself placed, with exactly its top three blocks leafed and everything below bare.

**`nether_forest_vegetation`** (`NetherForestVegetationConfiguration { state_provider: BlockStateProvider, spread_width: i32, spread_height: i32 }`, HIGH confidence — cross-checked against the reference). The attempt count, `spread_width * spread_width`, is correct. The nylium gate is a SINGLE up-front test on the block below `origin` (zero draws, not a per-candidate floor-tag test). Each attempt draws SIX values, not three: `dx = random.next_int_bounded(spread_width) - random.next_int_bounded(spread_width)`, `dy = random.next_int_bounded(spread_height) - random.next_int_bounded(spread_height)`, `dz = random.next_int_bounded(spread_width) - random.next_int_bounded(spread_width)` — two draws per axis, in the argument order X, then Y, then Z, never a single `next_int_bounded(spread*2+1) - spread` per axis. `sample_block_state_provider(&config.state_provider, random, resolver)` is evaluated BEFORE the placement tests, for EVERY attempt (fresh sample per attempt, not merely per successful candidate); the candidate is then accepted when it is empty, above the world's minimum Y, and the sampled state can survive there.

**`seagrass`** (`ProbabilityConfiguration { probability: f32 }`, HIGH confidence — cross-checked against the reference). Consumes FOUR scatter draws BEFORE any placement test: `x = random.next_int_bounded(8) - random.next_int_bounded(8)`, `z = random.next_int_bounded(8) - random.next_int_bounded(8)`, with Y taken from the `OCEAN_FLOOR` heightmap at that offset column — the feature never acts at `origin` itself. The water test (`props.is_still_water`) is applied at that SCATTERED position, not at `origin`. `is_tall = random.next_double() < config.probability` (ONE draw, using the config's own codec-supplied `probability` field, never a hardcoded `0.3`). If `is_tall`: place `seagrass[half=lower]` at the scattered position, `seagrass[half=upper]` one block above iff that position is also water; else place a single non-tall `seagrass` block at the scattered position.

**`kelp`** (no config, `NoOpConfiguration`-shaped, HIGH confidence — cross-checked against the reference; there is NO nested `next_int_bounded` call). Y is taken from the `OCEAN_FLOOR` heightmap at `origin`'s own column, and that position must be water (zero draws; no separate "solid floor" test beyond being on the ocean floor). `height = 1 + random.next_int_bounded(10)` (a SINGLE draw, not a nested `next_int_bounded(next_int_bounded(10) + 1)`). The column loop runs `h` from `0` through `height` inclusive, requiring both the current position and the one above to be water; at `h == height` it places `kelp[age=random.next_int_bounded(4)+20]` (an EXTRA draw), otherwise `kelp_plant`; if the water check fails at some `h > 0`, a `kelp` head is instead placed one block BELOW (with another `next_int_bounded(4)+20` draw) and the column stops there.

**`sea_pickle`** (`SeaPickleConfiguration { count: crate::data::IntProvider }` — this blueprint's designated second exact hand-trace, Acceptance tests). `config.count` is the number of ATTEMPTS, not a pickles value: `attempts = sample_int_provider(&config.count, random)` (1+ draws, provider-dependent — for the hand-trace's own `IntProvider::Uniform{min:0,max:3}` config this becomes exactly ONE `next_int_bounded`-derived draw, Context §0's `Uniform` row) is sampled ONCE, then looped that many times. EACH attempt independently draws its own scatter offset (`x = random.next_int_bounded(8) - random.next_int_bounded(8)`, `z` likewise, Y from `OCEAN_FLOOR`) and its own `pickles = 1 + random.next_int_bounded(4)` (a FRESH draw per attempt, not a shared `(1+raw).clamp(1,4)` derived from the attempt count); place `sea_pickle[pickles=pickles]` at that attempt's own position iff it is WATER (not air-or-water) and the pickle state can survive there.

**`coral_tree` / `coral_mushroom` / `coral_claw`** (SHARED `CoralFeatureConfiguration { state: crate::data::BlockStateSpec }`, one function per shape). Every coral shape FIRST draws a random coral block from the coral-blocks tag (ONE draw, via a random-element-of-tag helper) BEFORE any placement test; the gate at each candidate position is that the target is water OR an already-coral-tagged block AND the block ABOVE it is water — there is no solid-floor test at all. **`coral_tree`** (HIGH confidence): `trunk_len = 1 + random.next_int_bounded(3)` (ONE draw, 1–3, this part holds); straight `state`-block column for `trunk_len` layers; THEN `n_branches = 2 + random.next_int_bounded(3)` (ONE draw, 2 to 4, never a fixed two); branch directions come from a Fisher-Yates shuffle of the four horizontal directions (three further draws); per branch, `branch_height = 2 + random.next_int_bounded(5)` (ONE draw) and the branch walks upward, drawing `random.next_float() < 0.25` (one draw per step) to decide each further outward step once the branch has grown to length 2. **`coral_mushroom`** (HIGH confidence): FOUR upfront draws, `height = 3 + random.next_int_bounded(3)`, `width = 3 + random.next_int_bounded(3)`, `length = 3 + random.next_int_bounded(3)` (each 3 to 5, but the fill loop is inclusive of both endpoints, so the walked box spans 4 to 6 positions per axis), `sink_value = 1 + random.next_int_bounded(3)`; the retained shape is the box's own outer shell minus its edges and corners, sunk by `sink_value`; each surviving candidate additionally draws `random.next_float() < 0.1` as a hole roll before placing. **`coral_claw`** (HIGH confidence): a coral block is placed at `origin` first (with its own draws); `claw_direction = random.next_int_bounded(4)` is drawn BEFORE the arm count; `n_branches = 2 + random.next_int_bounded(2)` (ONE draw, 2–3, this part holds); arm directions come from a shuffle of `{claw_direction, its clockwise neighbor, its counter-clockwise neighbor}` (further draws); per arm, `sideway_length = 1 + random.next_int_bounded(2)`, then either `inway_length = 2 + random.next_int_bounded(3)` for the claw's own direction or a drawn choice between `{branch_direction, Up}` plus `inway_length = 3 + random.next_int_bounded(3)` otherwise, followed by a sideways walk and an inward walk of up to `inway_length` steps, each drawing `random.next_float() < 0.25` to rise one block.

### K. Java → Rust porting-pitfall checklist (this blueprint's own additions to M5-B07's Context §P)

1. **Not every gating roll is conditional, and not every conditional roll gates the same scope** — `beehive` and `cocoa` each draw ONE probability roll that gates their WHOLE rest of the decorator (never a per-log or per-direction roll), while `trunk_vine`'s own per-direction roll is UNCONDITIONAL (drawn regardless of the neighbor's air state) and only the PLACEMENT decision is conditional on it; `vegetation_patch`'s own edge-column roll is conditional on both being a non-corner boundary column AND the configured chance being non-zero. Assuming a uniform "gate then conditionally draw more" shape across all of these desyncs the very next feature's own RNG stream (the identical hazard M5-B07 Context §N.1 already named for `ore`'s own discard-on-air-exposure roll).
2. **A configured `IntProvider`'s sampled value is often an ATTEMPT COUNT or a length-ADJUSTMENT INPUT, not the final in-game value directly** — `sea_pickle`'s own `count` field is the number of independent scatter attempts, each drawing its OWN fresh `pickles` value, not a single draw whose result becomes the placed `pickles` property; `fallen_tree`'s own `log_length` provider is sampled and then reduced by 2 before use. Treating either provider's raw sampled value as the final output silently changes both the placed result and every downstream RNG draw.
3. **The multi-attachment tree driving loop is depth-first per attachment** (Context §H) — exactly the same hazard M5-B07 Context §F names for placement modifiers, restated here for the trunk-placer → foliage-placer boundary specifically: every attachment's own foliage draws complete before the next attachment's own call begins.
4. **`FoliageAttachment` is a genuinely new return shape** (Context §G) — a trunk placer that returns a single position (as M5-B07's own two kinds effectively did) where this blueprint's kinds need multiple silently drops canopy coverage at every attachment but the first, producing a plausible-but-wrong-looking tree with no compiler signal.
5. **This blueprint's own `root_placer` (Context §E) and M5-B12c's own `root_system` Feature kind are different mechanisms with a confusingly similar name** — never conflate the two when reading either blueprint (Context §E's own explicit note).
6. **`driver.rs`'s per-feature call site is edited by three blueprints in sequence** (M5-B07 defines it, M5-B12e changes it, this blueprint changes it again) — apply them in that order; this blueprint's own Deliverables text for that file is the final, correct end state (Context §D).

### Claims to verify (TEST-D57)

- WorldgenRandom overrides next(bits), fork, forkPositional and setSeed, but never next_int_bounded itself; next_int_bounded always resolves to the one backend-independent implementation, whose power-of-two fast path runs first: when bound is a power of two it returns the high bits of a 31-bit draw with no loop, and only for a non-power-of-two bound does it fall back to the classic rejection loop (bits = next_bits(31); val = bits % bound; loop until (bits - val + bound - 1) >= 0 in wrapping 32-bit arithmetic).
- next_int_between_inclusive(min, max_inclusive) equals next_int_bounded(max_inclusive - min + 1) + min, consumed as one draw.
- next_float() is uniform on [0.0, 1.0), computed as next_bits(24) as f32 * 2^-24, one draw.
- next_bool() is next_bits(1) != 0, one draw.
- next_double() is uniform on [0.0, 1.0), implemented as two next_bits calls internally but counted as one logical draw.
- IntProvider::Constant(n) samples to n with zero RNG draws; IntProvider::Uniform{min,max} samples via next_int_between_inclusive(min,max), one draw.
- HeightProvider has six kinds: Constant, Uniform, BiasedToBottom, VeryBiasedToBottom, Trapezoid, and WeightedList.
- BlockStateProvider::SimpleStateProvider consumes zero RNG draws; WeightedStateProvider consumes one draw to make a cumulative-weight selection among its entries.
- The shared tree-height formula is height = base_height + random.next_int_bounded(height_rand_a + 1) + random.next_int_bounded(height_rand_b + 1), two draws, with the height_rand_a draw strictly before the height_rand_b draw.
- BlobFoliagePlacer samples radius outside the foliage placer entirely (drawn once by the tree driver before the root and trunk placers run) and offset once per FoliageAttachment inside the foliage placer (two draws overall, at different call sites, not two draws local to the placer), then per layer fills a SQUARE footprint (not a disk) with a probabilistic corner-skip that consumes one next_int_bounded(2) draw per exact-corner candidate block.
- Real vanilla's placement context carries an Optional<PlacedFeature> for the currently-placing top-level feature, not a feature id, populated only at the true top-level entry point; a nested re-entry passes an empty Optional, so a Biome{} placement modifier encountered inside a nested chain throws in real vanilla rather than checking any feature's presence in the current biome's step list, and vanilla data packs simply never nest a Biome{} modifier this deep.
- The Chorus Plant (the chorus_plant feature) grows only in the End dimension in real vanilla.
- The complete named vanilla Feature registry this project recognizes comprises 63 kinds: 6 already implemented (ore, disk, spring_feature, lake, tree, simple_block), 17 owned by this blueprint (fallen_tree, huge_red_mushroom, huge_brown_mushroom, vines, vegetation_patch, waterlogged_vegetation_patch, seagrass, kelp, coral_tree, coral_mushroom, coral_claw, sea_pickle, bamboo, huge_fungus, nether_forest_vegetation, weeping_vines, twisting_vines), 35 owned elsewhere, and 5 End-exclusive kinds out of scope (chorus_plant, end_platform, end_spike, end_island, end_gateway).
- random_patch is not a real vanilla feature kind: it registers no Feature id, has no configuration type, and appears nowhere in the vanilla data pack; the research corpus's own 63-name enumeration is the correct and complete universe of confirmed vanilla feature kinds.
- Only mangrove-style trees carry a root system (RootPlacer) in real vanilla; straight and bending trunk placers never carry one.
- Mangrove root placement has no soil check on the block directly below the tree's origin; the mud/muddy-mangrove-roots requirement is a placement-modifier gate on the placed feature (a would_survive-style predicate keyed on the tree's real support-block tag, broader than mud alone), applied before the root placer ever runs. The root placer itself walks the column from origin up to the shifted trunk origin, requiring every position to be air/replaceable-by-trees or can_grow_through-tagged; if any position fails, the entire tree placement is a no-op.
- Mangrove root placement samples a trunk_offset_y value and shifts the real trunk placer's origin upward by that amount from the root placer's own origin, since roots grow down from the tree's nominal base into the mud below while the visible trunk starts above it.
- A mangrove tree always starts exactly four root strands, one per fixed horizontal direction in North, East, South, West order, each rooted at the shifted trunk origin's own neighbor in that direction with zero positional-jitter draws; max_root_width is not a strand count but a Manhattan-distance bound from the root origin, tested against each candidate as the recursive walk proceeds.
- During a mangrove root strand's recursive walk, a candidate whose distance from the root origin exceeds max_root_width advances straight down with zero draws; otherwise one skew_roll = random.next_float() is drawn against random_skew_chance, and depending on the distance band this either yields both a below and a sideways-then-below candidate, or gates a second random.next_bool() draw choosing between a sideways and a below candidate — the sideways direction is always the strand's own already-established direction, never a freshly drawn random.next_int_bounded(4).
- A mangrove root strand's recursive walk does not stop early on a non-placeable candidate; such a candidate is simply skipped and the recursion continues to the next one. The only real early exit is a layer/length guard (reaching max_root_length), which returns failure and aborts the entire tree, not just that strand.
- Mangrove root placement never contributes foliage attachment points; a tree's returned FoliageAttachments come only from its trunk placer.
- This blueprint's own RootPlacer (used only by mangrove-style trees, Context §E) and M5-B12c's own root_system Feature kind are two different vanilla mechanisms that happen to share a similar-sounding name; RootPlacer is a tree-configuration component while root_system is a standalone Feature (used for e.g. flowering azalea trees), and the two do not interact.
- The vanilla RootPlacer registry comprises exactly one kind, mangrove_root_placer, unimplemented anywhere else in the project before this blueprint.
- TreeDecorators run, in their declared list order, strictly after the trunk, foliage, and (if present) root placer have all finished writing blocks.
- The vanilla TreeDecorator registry comprises 10 kinds, of which this blueprint implements 6 (beehive, trunk_vine, leave_vine, cocoa, attached_to_leaves, attached_to_logs) and leaves 4 named-deferred (pale_moss, creaking_heart, alter_ground, place_on_ground).
- The fixed horizontal-direction table maps direction indices 0, 1, 2, 3 to (dx, dz) offsets (0,-1), (1,0), (0,1), (-1,0), corresponding to North, East, South, and West respectively.
- For the beehive tree decorator, a probability roll (random.next_float(), one draw) gates the entire rest of the decorator; on success, nothing resembling a single direction-index draw follows — instead a hive Y is derived from the lowest log/leaf positions (drawing an extra random.next_int_bounded(3) only when there are no leaves), a candidate list is built from every log at that Y offset one step in each of three fixed directions (East, South, West), and that list is Fisher-Yates shuffled before a survivor is chosen.
- On a successful beehive probability roll, the candidate bee-nest position is the first shuffled candidate — a log at the derived hive Y offset exactly one step (not two) in one of three fixed directions (East, South, West, never North) — that survives the placement test; no direction index is ever drawn.
- A bee nest is placed at the beehive candidate position only if that position is strictly air (not air-or-replaceable) and the position one step South of it (the hive's fixed facing direction, not a leaf test) is also strictly air.
- In real vanilla, the bee_nest block carries block-entity NBT tracking its occupant bees and honey level beyond the placed block's own facing state, a payload this blueprint's beehive decorator does not populate since it places the bare block state only.
- For the trunk_vine decorator, for each log position and each of the four cardinal directions in fixed West, East, North, South order, a vine-placement roll (random.next_int_bounded(3), one draw) is drawn unconditionally, before the neighbor's air state is tested; a vine is placed on that face iff the roll is greater than 0 (probability 2/3) and the neighbor is air, and its facing property is the opposite of the offset direction.
- The leave_vine decorator shares only its four unrolled direction blocks with trunk_vine: each direction's roll is a random.next_float() against a codec-configured probability (not random.next_int_bounded(3)), and on success the vine is additionally extended downward through up to 4 further air positions, a behavior trunk_vine has no counterpart for.
- For the cocoa decorator, a single probability roll (random.next_float(), one draw) is drawn once for the whole decorator, before any log is visited, not once per log; on success, for each log within 2 of the lowest log's Y and each of the four cardinal directions, an unconditional random.next_float() draw is compared against a fixed 0.25 (not the configured probability), and on success a cocoa pod is placed on the opposite face at an age drawn via random.next_int_bounded(3), not a fixed age of 0.
- For attached_to_leaves, the leaf list is shuffled first (one draw per element), then per leaf a direction index is drawn unconditionally from a required, non-empty configured directions list (no all-four-cardinals default) before a probability roll is drawn (short-circuited away for a position already inside an exclusion box grown around each already-placed propagule, not around the tree's origin); for attached_to_logs, which has no exclusion-radius or required-empty-blocks fields at all, the log list is shuffled first, then per log a direction index and a probability roll are drawn and gate a single air test at the offset position, with no multi-step walk.
- The vanilla TrunkPlacer registry comprises exactly 9 kinds: straight_trunk_placer and bending_trunk_placer (already implemented) plus forking_trunk_placer, giant_trunk_placer, mega_jungle_trunk_placer, dark_oak_trunk_placer, fancy_trunk_placer, cherry_trunk_placer, and upwards_branching_trunk_placer, the last of which is named-deferred (it is exactly the kind both mangrove configured features use).
- forking_trunk_placer is a leaning main trunk plus one conditional fork, not a straight column plus two symmetric branches: it draws a lean direction, a lean height, and a lean count (three draws) and drifts the main column by that lean before emitting one FoliageAttachment at its own top; it then draws a second direction index unconditionally, and only forks (drawing a branch position and a branch length of 1 + random.next_int_bounded(3), i.e. 1 to 3 logs, and emitting a second FoliageAttachment) if that second direction differs from the lean direction.
- giant_trunk_placer places four below-trunk blocks, then stamps a 2x2 log footprint for every Y layer of the drawn height EXCEPT the top layer, which keeps only a single column; it consumes zero draws beyond the two height draws, and returns exactly one FoliageAttachment one block above the top trunk layer with double_trunk true.
- mega_jungle_trunk_placer calls giant_trunk_placer's own routine first, then adds a whole additional randomized branch loop giant_trunk_placer does not have, drawing a starting branch height, a per-iteration decrement, and a per-iteration angle, and emitting one further FoliageAttachment with double_trunk false per branch.
- dark_oak_trunk_placer draws a lean direction, a lean height, and a lean count (three draws, the same lean shape as forking_trunk_placer) and stamps the 2x2 footprint only where the base position is air-or-leaves, returning exactly one FoliageAttachment with double_trunk true at the top layer itself (not y+1); it then scans a 4x4 ring of 12 cells around the inner 2x2, each drawing a next_int_bounded(3), and only when that roll is at most 0 draws a branch length and stamps a downward branch with its own FoliageAttachment (double_trunk false) — 1 to 13 attachments total, never a fixed two.
- fancy_trunk_placer draws no branch count, direction index, or branch length at all: trunk_height is the golden-ratio-scaled floor of height+2 (not the raw drawn height), and branches instead come from a descending scan of candidate Y layers, each of up to a computed number of cluster attempts, with each attempt drawing a radius float and an angle float (two draws) to place a branch's own start offset; branch length falls out of the branch's own walk toward its computed endpoint rather than being sampled.
- fancy_trunk_placer's branches do not start at a deterministic evenly-spread Y; each branch's base Y is computed from its own randomized start offset and is then trimmed against a height-proportional retention threshold, so the vertical spread is driven by the randomized descending-layer scan, not a zero-draw formula.
- cherry_trunk_placer always places exactly two branches, not branch_count-many: both branch start offsets are sampled first, before anything else; branch_count is sampled next but only feeds trunk_height's own derivation; exactly one direction index is drawn for the whole placer, with the second branch using its opposite; and each branch draws its own end-offset and horizontal-length providers, then walks toward its endpoint drawing a float at every step to choose a vertical or horizontal step, a drooping staircase rather than a flat-Y horizontal run.
- The multi-attachment foliage driving loop calls the foliage placer once per FoliageAttachment the trunk placer returned, in the same order the trunk placer produced them, with every attachment's own draws completing fully before the next attachment's call begins.
- The vanilla FoliagePlacer registry comprises exactly 11 kinds: blob_foliage_placer and spruce_foliage_placer (already implemented) plus pine_foliage_placer, acacia_foliage_placer, bush_foliage_placer, fancy_foliage_placer, jungle_foliage_placer, mega_pine_foliage_placer, dark_oak_foliage_placer, cherry_foliage_placer, and random_spread_foliage_placer, the last of which is named-deferred (it is what the azalea tree's configured feature uses).
- pine_foliage_placer draws four values, not two: a base radius provider sample plus a second next_int_bounded draw added to it, an offset, and its own foliage-height provider sample; its per-layer radius is a ramp-up-then-ramp-down profile driven by those draws, not a fixed even/odd alternation, filling a square footprint (that part holds) with a deterministic, zero-draw exact-corner skip whenever the current radius is above zero.
- acacia_foliage_placer draws radius0 and offset0 (two draws total, its own height field being a fixed zero) but places THREE placeLeavesRow-style layers, not one; only the topmost (y == 0) layer carves a plus/cross shape via a deterministic edge test, while the other two layers use Blob's own deterministic exact-corner skip.
- bush_foliage_placer shares Blob's draw shape exactly (two provider draws plus one next_int_bounded(2) per exact-corner candidate) but not its geometry: its per-layer radius formula omits Blob's own halving of the layer offset and its own zero-clamp, and its skip test omits Blob's additional y == 0 clause, so it keeps that layer's corners half the time where Blob always removes them.
- jungle_foliage_placer draws an extra value beyond radius and offset when its attachment is not double-trunked, uses a per-layer radius formula that grows rather than shrinks toward the top, and uses a true circular skip test with zero draws, never Blob's next_int_bounded(2) corner roll.
- fancy_foliage_placer draws radius0 and offset0 (two draws total, its own height field being a fixed constant), then for each layer sets the radius to radius0 on the first and last layers and radius0+1 in between, and keeps only positions where (dx+0.5)^2 + (dz+0.5)^2 is at most the radius squared, not dx*dx + dz*dz <= layer_radius*layer_radius + 1.
- mega_pine_foliage_placer draws radius0, offset0, and crown_height (three draws total), but has no constant-radius crown band and no even/odd alternation: its radius grows toward the bottom of the crown via a floor-scaled term with a jagged +1 bump, using the same circular skip test as jungle_foliage_placer.
- dark_oak_foliage_placer's radius provider is sampled exactly once per tree by the tree driver itself, not per attachment; only offset is sampled per call (one draw, not two), and its own height field is a fixed constant. It is called once per each of the trunk placer's 1 to 13 attachments (never a fixed two): a double-trunk attachment places three rows plus a coin-flip-gated fourth row, while every other attachment places two rows.
- cherry_foliage_placer draws radius0, offset0, and its own foliage-height provider sample (three draws, not two) and uses no integer rolls at all: its layer stack is two tapered rows, a run of full-radius rows, and two hanging-leaves-below rows, and its skip test draws only floats — a wide layer drops an exact corner unconditionally with no draw and rolls a near-corner band instead, while only a narrow layer rolls the exact corner itself.
- A FoliageAttachment's double_trunk flag is true only for attachments produced above a 2x2 trunk footprint (giant_trunk_placer, mega_jungle_trunk_placer, dark_oak_trunk_placer), meaning the foliage placer must center its canopy across all 4 trunk columns at that attachment rather than a single column.
- chorus_plant, end_platform, end_spike, end_island, and end_gateway are End-dimension-exclusive vanilla features.
- void_start_platform, despite its End-adjacent name, is usable outside the End dimension in real vanilla.
- fallen_tree places an unconditional stump log at origin first, then draws a direction index (one draw) and a log length via its configured IntProvider minus 2 (one or more draws), then draws a further start-offset roll (one draw) placing the log's own start 2 or 3 blocks out from origin; placement is accepted as long as at most 2 consecutive non-solid support columns occur along the log's length, not a single solid-block-below-origin gate.
- huge_fungus always requires the block below origin to match the configured valid_base_state, regardless of the planted flag (planted only skips a separate generation-depth ceiling check); it draws total_height = 4 + random.next_int_bounded(9) (one draw, 4 to 12), doubles it when random.next_int_bounded(12) == 0 (a second draw), and forms a hat whose height and per-layer radius are themselves drawn (further draws) rather than a fixed-radius-3 disk, with each hat-candidate block drawing two separate next_float() rolls (a decor roll then a hat-fill roll), never a single random.next_int_bounded(5) == 0 roll.
- huge_red_mushroom and huge_brown_mushroom draw height = 4 + random.next_int_bounded(3) (one draw, 4 to 6), then double it when random.next_int_bounded(12) == 0 (a second draw, giving 4-6 or 8-12), place the cap before the stem, and place a straight stem column for the full height layers, not height-1.
- The huge mushroom cap is not a uniform 3-layer shape for both kinds: huge_red_mushroom's cap spans 4 Y-layers (height-3 through height inclusive, no clamp at 0) while huge_brown_mushroom's cap is a single layer at height; both are fully deterministic given height for a non-drawing cap_provider, but a weighted cap_provider would draw once per candidate position.
- huge_red_mushroom's cap keeps only an edge ring (exactly one of the X/Z edge flags set) with corners already excluded on its three lower layers, and is solid on its narrower top layer (radius foliage_radius - 1); huge_brown_mushroom's cap is a single flat disk at the SAME radius as red's lower layers (not one larger), with its four exact corners notched (both edge flags set), the geometry essentially inverted relative to the original description.
- vegetation_patch and waterlogged_vegetation_patch draw x_radius and z_radius as two independent samples of xz_radius (each plus 1), X before Z, then scan candidate columns within that rectangle in ascending dx then dz order.
- For vegetation_patch and waterlogged_vegetation_patch, boundary columns use a rectangular (not circular) edge test; the four corners are skipped unconditionally with zero draws, and only an edge-but-not-corner column draws an edge_roll (one draw), short-circuited away entirely when extra_edge_column_chance is 0; interior columns draw nothing.
- For vegetation_patch and waterlogged_vegetation_patch, each surviving column moves inward while air then outward while non-air (zero draws) and is accepted when empty with a sturdy inward-facing neighbor, never scanning from origin.y + vertical_range for the replaceable_tag directly; the waterlogged variant does not require pre-existing water, it keeps positions not exposed on four sides and down, then sets water into them.
- For vegetation_patch and waterlogged_vegetation_patch, a depth IntProvider is sampled (one or more draws) plus possibly one more via a next_float() roll gated by extra_bottom_block_chance (short-circuited away at chance 0), and ground_state blocks are then written starting one step further inward of the found surface position, never at the surface position itself.
- vegetation_patch and waterlogged_vegetation_patch re-enter the configured vegetation_feature as a full nested placement-modifier chain at the computed surface position.
- The ordinary overworld climbing vines feature consumes zero RNG draws and touches exactly one position: it checks the six directions in fixed Down, Up, North, South, West, East order, skipping Down, and places a single vine face on the FIRST direction whose neighbor is acceptable, never a 9x9 footprint and never a next_bool() roll.
- weeping_vines is genuinely config-free and hardcodes its own native block ids, but twisting_vines is NOT non-datapack-configurable: it carries its own spread_width/spread_height/max_height config, read at placement time.
- weeping_vines and twisting_vines are both multi-hundred-attempt scatter features, not a single column at origin: each runs many scatter attempts across a footprint bounded by its own spread parameters, and each successful hit draws its own vine height and places body segments plus a distinct head block whose age is drawn separately, rather than a shared height = 8 + random.next_int_bounded(4) formula applied to one column.
- bamboo draws stalk_height = 5 + random.next_int_bounded(12) (one draw, 5 to 16) first and unconditionally; the probability roll (random.next_float(), one draw) comes after and gates only a separate podzol disk, not the bamboo growth. Leaves are not thirds: only when the column reaches at least 3 blocks does it get a leafed top of exactly 3 blocks (one new block placed above the last trunk block plus the two trunk blocks just below it rewritten in place), with everything below staying bare.
- nether_forest_vegetation's nylium gate is a single up-front test on the block below origin, not a per-candidate floor-tag test; each attempt draws six values, not three, two per axis in the order X then Y then Z, and samples the configured block state before testing placement, for every attempt, not merely successful candidates.
- seagrass draws four scatter values (x and z, two draws each) before any placement test, taking Y from the OCEAN_FLOOR heightmap at that offset column rather than acting at origin itself; is_tall is gated by the config's own codec-supplied probability field, never a hardcoded 0.3.
- kelp uses a single random.next_int_bounded(10) draw, never a nested random.next_int_bounded(random.next_int_bounded(10) + 1); Y comes from the OCEAN_FLOOR heightmap and that position must be water, with no separate solid-floor test.
- sea_pickle's configured IntProvider is the number of placement ATTEMPTS, not a pickles value; each attempt independently draws its own scatter offset and its own pickles = 1 + random.next_int_bounded(4) (a fresh draw per attempt), and the gate is that the position is WATER (not air-or-water) with a survivable pickle state.
- coral_tree, coral_mushroom, and coral_claw all first draw a random coral block from the coral-blocks tag (one draw) before any placement test, and their shared gate is that the target is water or an already-coral-tagged block with water above it — there is no solid-floor test at all.
- coral_tree's branch count is 2 + random.next_int_bounded(3) (2 to 4, never a fixed two), its branch directions come from a shuffle of the four horizontal directions (three further draws), and each branch draws its own height and a per-step outward-growth roll, consuming far more draws than the trunk-length roll alone.
- coral_mushroom draws four values up front (height, width, length each 3 + random.next_int_bounded(3), and a sink value), walks a box 4 to 6 positions per axis (inclusive loop bounds over 3-to-5 drawn dimensions) keeping only its outer shell minus edges and corners, and rolls an additional hole chance per surviving candidate — never a fixed radius-2, 2-layer, zero-draw shape.
- coral_claw draws a claw direction before the arm count, shuffles its own arm directions from the claw direction and its two neighbors, and draws a sideways length plus an inward length and a per-step rise roll for every arm — consuming far more draws than the arm-count roll alone.

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

1. `giant_trunk_placer_hand_trace` — `WorldgenRandom::new(AnyRandom::new_legacy(777))`, `TrunkPlacerJson::GiantTrunkPlacer{base_height:5, height_rand_a:3, height_rand_b:2}`, `origin=(0,64,0)`: hand-traced against M5-B01's own published `next_int_bounded` formula (draw 1's bound, `4`, IS a power of two and takes the fast path, `((bound as i64) * next_bits(31) as i64) >> 31`; draw 2's bound, `3`, is not, and takes the classic rejection loop — computed once by hand for this blueprint's own derivation pass): draw 1 (`next_int_bounded(4)`) = `2`, draw 2 (`next_int_bounded(3)`) = `0`, `height = 5+2+0 = 7`. Assert `place_trunk` returns `(7, vec![FoliageAttachment{pos:(0,71,0), radius_offset:0, double_trunk:true}])` (one block above the topmost trunk layer) and that `world`'s recorded `set_block` calls are exactly the 29 positions: the 4 below-trunk positions `{(0,63,0),(1,63,0),(0,63,1),(1,63,1)}`, the single always-stamped column `(0,y,0)` for `y in 64..=70` (7 positions), and the three additional 2×2 columns `{(1,y,0),(0,y,1),(1,y,1)}` for `y in 64..=69` only — the top layer, `y=70`, keeps just the single `(0,70,0)` column (18 positions) (order: this blueprint's own `place_trunk` implementation's own deterministic column/layer iteration order — asserted as a literal expected `Vec<BlockPos>`, a regression pin on THIS blueprint's own algorithm, not a vanilla-verified value, per this project's established convention).
2. `forking_trunk_placer_consumes_six_draws` — fixed seed; instrument draw-count via RNG-state comparison against an independently-computed 6-draw reference sequence (2 height + 2×2 branch draws); assert exactly 6 draws consumed and exactly 2 `FoliageAttachment`s returned, at different positions.
3. `mega_jungle_trunk_placer_extends_giant_shape` — same fixed seed and config field values fed to both `GiantTrunkPlacer` and `MegaJungleTrunkPlacer`; assert `MegaJungleTrunkPlacer`'s own recorded `set_block` calls are a SUPERSET of `GiantTrunkPlacer`'s own recorded calls (the identical below-trunk writes and 2×2-minus-top-layer stamp), that `MegaJungleTrunkPlacer` returns `GiantTrunkPlacer`'s own single `double_trunk: true` attachment PLUS zero or more additional `radius_offset: -2, double_trunk: false` branch attachments, and that the two are NOT byte-identical overall (proves the "extends, does not merely reparametrize" correction, Context §G).
4. `dark_oak_trunk_placer_returns_one_double_trunk_plus_ring_attachments` — structural: exactly ONE returned `FoliageAttachment` has `double_trunk: true` (at the top trunk layer itself, not `y+1`), and every other returned attachment (0 to 12 of them) has `double_trunk: false`; across 50 fixed seeds, total attachment count is always in `[1,13]`.
5. `fancy_trunk_placer_no_single_branch_count_draw` — for 200 fixed seeds, instrument the RNG-state trace and assert it never contains a `next_int_bounded(3)` draw immediately following the height draws (ruling out a `3 + next_int_bounded(3)` branch-count draw, Context §G); separately assert `trunk_height == ((height + 2) as f32 * 0.618).floor() as i32` for each seed (the golden-ratio split applies to `height + 2`, not the raw drawn `height`).
6. `cherry_trunk_placer_always_returns_two_attachments` — for 50 fixed seeds and varying `branch_count: IntProvider::Constant(n)` for `n` in `2..=5`; assert `place_trunk` returns exactly 2 `FoliageAttachment`s every time (branch_count feeds only `trunk_height`'s own derivation, never the branch count), and that the second branch's horizontal direction is the exact opposite of the first branch's single drawn `tree_direction`.
7. `bush_foliage_placer_draw_shape_matches_blob_but_geometry_differs` — same fixed seed, same `radius`/`offset`/`height` config fed to both `place_blob_foliage` (M5-B07's own already-shipped function) and `bush_foliage_placer`'s dispatch; assert IDENTICAL total draw counts (two provider draws plus one `next_int_bounded(2)` per exact-corner candidate) but assert the layer at `yo == 0` is NOT byte-identical between the two (Bush keeps that layer's corners on a `next_int_bounded(2) != 0` roll, Blob never does).
8. `jungle_foliage_placer_uses_circular_not_square_skip` — fixed seed, `radius: Constant(3)`, `attachment.double_trunk: false` (forcing the extra `1 + next_int_bounded(2)` leaf-height draw); assert every placed position satisfies `dx*dx + dz*dz <= current_radius*current_radius` and none is ever dropped by a `next_int_bounded(2)` corner roll (there is none), and that a `double_trunk: true` attachment consumes one fewer draw overall.
9. `acacia_dark_oak_foliage_corners_always_skipped` — for both kinds, across 50 fixed seeds, no placed block is ever at an exact corner of its own layer's bounding square.
10. `multi_attachment_foliage_driving_loop_is_depth_first_per_attachment` — a synthetic 3-attachment trunk result fed through `AcaciaFoliagePlacer` (2 draws/attachment); assert the SAME final RNG state results whether processed via this blueprint's own driving loop or via 3 independently-chained manual calls in attachment order (proves no interleaving, Context §K.3).

### `crates/worldgen/tests/vegetation_root_and_decorators.rs`

1. `mangrove_root_placer_requires_clear_shaft_to_trunk` — `FakeWorld` with a non-air/non-`can_grow_through` block somewhere in the column between `origin` and the shifted trunk origin; assert `TreeConfiguration::place` (with a `root_placer` present) writes ZERO blocks anywhere (the whole-tree no-op, Context §E) — no soil test on the block below `origin` is exercised by this test, since real vanilla has none.
2. `mangrove_root_placer_starts_four_fixed_direction_strands` — clear shaft, fixed seed, `random_skew_chance:0.0` (skew never triggers): assert exactly 4 strands are started, one per fixed direction in North/East/South/West order (Context §F's table), each beginning at the shifted trunk origin's own N/E/S/W neighbor with NO horizontal-jitter draw consumed; assert each strand walks straight down (no `sideways` step, since `random_skew_chance:0.0` never triggers the `next_bool` branch either) until `max_root_length` or a non-replaceable position is reached.
3. `beehive_probability_gate_is_the_only_draw_on_failure` — `probability: 0.0` (the gate always fails, since `next_float() >= 0.0` is always true); assert RNG state after the call reflects EXACTLY one draw consumed (the probability roll) and zero blocks placed.
4. `beehive_places_at_first_shuffled_survivor` — `probability: 1.0` (always succeeds); a `FakeWorld` with several candidate logs at the derived `hive_y`, exactly one of whose East/South/West-offset positions (and that position's own South neighbor) is air; assert a `bee_nest` block is placed at that survivor with `facing = south` always, regardless of which offset direction it came from.
5. `trunk_vine_and_leave_vine_have_different_draw_shapes` — same fixed seed, same synthetic single-log/single-leaf `TreePlacementLog` fed to both decorators; assert `trunk_vine` consumes exactly 4 unconditional `next_int_bounded(3)` draws (one per direction, regardless of air state) while `leave_vine` consumes 4 `next_float()` draws against its own configured `probability`, with a successful `leave_vine` face additionally extending downward through further air positions that `trunk_vine` never touches.
6. `cocoa_probability_gate_covers_the_whole_decorator` — `probability: 0.0` (the gate always fails); a `TreePlacementLog` with several log positions; assert RNG state after the call reflects EXACTLY one draw consumed (the single whole-decorator gate) and zero per-log/per-direction draws or placements.
7. `unsupported_decorator_kind_is_documented_no_op` — `TreeDecoratorJson::Unsupported` (via an unrecognized `type` string); assert `apply_tree_decorator` writes zero blocks and consumes zero draws, does not panic.

### `crates/worldgen/tests/vegetation_terminal_features.rs`

1. `sea_pickle_hand_trace` — `WorldgenRandom::new(AnyRandom::new_legacy(42))`, `SeaPickleConfiguration{count: IntProvider::Uniform{min:0,max:3}}` (a `Uniform` provider chosen specifically to land on exactly ONE draw via `next_int_between_inclusive`, Context §0): hand-traced draw = `2` (computed once by hand against M5-B01's own published `next_int_bounded` formula for this exact seed/bound — the bound here, `4`, IS a power of two, so this draw takes the fast path, `((bound as i64) * next_bits(31) as i64) >> 31`, not the rejection loop), `pickles = (1+2).clamp(1,4) = 3`; assert `sea_pickle[pickles=3]` is placed at `origin` (with a `FakeWorld` pre-seeded so the position/floor gate passes).
2. `kelp_inner_draw_happens_before_outer_bound_is_known` — instrumented via a `WorldgenRandom` wrapper that records each `next_int_bounded` call's OWN `bound` argument in call order; assert the recorded sequence is `[10, X]` where `X` is whatever the FIRST draw's own result was `+1` (proving the inner `nextInt(10)` call's result determines the second call's bound, Context §K.2).
3. `huge_mushroom_red_vs_brown_cap_shape_differs` — same fixed seed, same `HugeMushroomConfiguration`, dispatched once as `is_red:true` and once `is_red:false`; assert the red cap's TOP-layer exact-corner positions ARE present (filled solid) while its three LOWER layers' exact-corner positions are absent (an edge-ring-minus-corners shape); assert the brown cap's single-layer exact-corner positions are absent (notched) — the corner behavior is inverted relative to a naive "red rounds, brown is flat" guess (Context §J).
4. `vegetation_patch_recurses_into_nested_placed_feature` — mirrors M5-B07's own `random_patch` recursion test exactly (a trivial `simple_block`-configured nested feature); asserts the nested feature's own block actually appears at the computed surface position, not merely that candidacy was validated.
5. `waterlogged_vegetation_patch_sets_water_rather_than_requiring_it` — same fixture as test 4 but with a non-water, non-exposed surface column; assert the ground position is accepted and water is SET there regardless of its prior (non-water) state, and that a column exposed on one of North/East/South/West/Down is instead rejected (the real gate, Context §J).
6. `weeping_vines_and_twisting_vines_grow_opposite_directions` — same fixed seed, `TwistingVinesConfig{spread_width:2, spread_height:2, max_height:8}`; assert `weeping_vines`' every placed body/head position has Y at or below its own scatter origin (ceiling-hung, grows down) while `twisting_vines`' every placed position has Y at or above its own scatter origin (floor-based, grows up) — a structural, per-attempt directional check, not a shared single-column height outcome.
7. `bamboo_height_draw_precedes_and_is_independent_of_probability_roll` — `probability: 0.0`; assert `stalk_height` is still drawn AND the stalk still grows (a bamboo column is still placed) even though the probability roll fails, since that roll gates only the podzol disk; assert the podzol disk itself is NOT placed.
8. `coral_family_draws_a_coral_block_before_the_gate` — for all three coral kinds, a non-water `FakeWorld` column; assert zero placements but assert exactly one RNG draw is consumed regardless (the coral-block-from-tag draw, taken before any placement test).
9. `nether_forest_vegetation_try_count_is_spread_width_squared` — `spread_width: 3`; instrument candidate-position draw sextuples (two draws per axis, X then Y then Z); assert exactly 9 candidates are evaluated and each draws its own block-state sample before its own placement test.
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

(g) **Several small, explicitly-named gaps are not resolved by editing `M5-B00-index.md`, `M5-B07-features-decoration.md`, `M5-B08-structures.md`, or any M5-B12a-e file** — all are outside this blueprint's assigned path. `chorus_plant` (Context §A/§B) is unimplemented anywhere in this project — harmless under GEN-D1's own End-out-of-scope framing (no corpus chunk exercises it), but named here rather than silently left inconsistent. `upwards_branching_trunk_placer` and `random_spread_foliage_placer` (Context §G/§H) are likewise unimplemented anywhere in this project — NOT harmless in the same way, since `upwards_branching_trunk_placer` is the trunk placer both mangrove configured features actually use, meaning this blueprint's own `RootPlacer`/mangrove work (Context §E) cannot drive a real end-to-end mangrove tree until a future blueprint ships it; named here rather than silently left inconsistent. `random_patch` — one of M5-B07's own originally-claimed 7 implemented kinds — is not a real vanilla feature kind at all (Context §B); that is an M5-B07-owned fact outside this blueprint's edit scope. (`blueprints/M5/M5-B00-index.md` owns the ID-reservation history and the full 63-kind coverage audit; it is not restated or re-litigated here.)

(h) **`driver.rs`'s per-feature call site is a three-blueprint sequence, not a single edit** (Context §D/§K.6) — this blueprint's own Deliverables text for that file is the correct final state, superseding M5-B12e's own intermediate one; an implementer who applies only M5-B07+the M5-B12 family (without this blueprint) has a completely valid, self-consistent intermediate state that simply lacks this blueprint's own 17 kinds, never a broken one.

(i) **No `unsafe` code.** Every function in this blueprint's Deliverables is implementable in safe Rust.

## Verification commands

- `cargo build -p rc-worldgen` — zero warnings.
- `cargo nextest run -p rc-worldgen` — every test in `vegetation_trunk_foliage.rs`, `vegetation_root_and_decorators.rs`, `vegetation_terminal_features.rs`, `vegetation_dispatch_composition.rs` passes.
- `cargo test --doc -p rc-worldgen` — exits 0.
- `cargo run -p xtask -- fmt-check` / `-- lint` / `-- lint-deps` — all exit 0.
- CI tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D37/D50).
