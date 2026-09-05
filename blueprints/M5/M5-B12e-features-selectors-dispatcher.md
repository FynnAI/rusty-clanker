# M5-B12e — Features: Selectors, Combinators & the Combined Dispatcher (Underground Tier 2, Part 5 of 5)

| Field | Content |
|---|---|
| ID | M5-B12e |
| Milestone | M5 — World Generation Parity |
| Prerequisites | M5-B12a, M5-B12b, M5-B12c, M5-B12d (this family's four sibling blueprints — every kind function this blueprint's own combined dispatcher registers is theirs, restated by reference, never re-derived). Transitively M5-B01, M5-B02, M5-B03, M5-B07, M5-B08. |
| Implements | GEN-D19 (features & placement — final of five blueprints closing M5-B07's own non-vegetation deferred backlog; this blueprint finishes the registration so the combined dispatcher is real, not partial), GEN-D6 (feature-seed call sites, unchanged mechanism), GEN-D20 (restated non-conflation), GEN-D16 (ore-vein/ore-feature boundary — NOT resolved by any reuse of M5-B07's own `ore::place`, which `scattered_ore` never calls; `scattered_ore` shares only the target-rule-plus-air-exposure admissibility check with the `ore` kind, so GEN-D16's boundary needs the planning role's own restatement). |
| Crates touched | `rc-worldgen` (`crates/worldgen/`) only: creates `src/decoration/underground/selectors.rs`; the final modification to `decoration/underground/mod.rs` (one new `pub mod` line PLUS the full body of `place_configured_feature_all`, which only this blueprint can write, since it is the first family member to have every sibling's kind functions available); one small, additive modification to M5-B07's already-specified `src/decoration/driver.rs` (Context §S). No `Cargo.toml` change. |
| Estimated scope | L. |

## Goal & Done definition

Close the final 7 of the 35 non-vegetation `Feature` kinds this family closes — `no_op`, `random_selector`, `weighted_random_selector`, `simple_random_selector`, `random_boolean_selector`, `sequence`, `scattered_ore` — and, because this is the last blueprint in the M5-B12 family to land, define the **real, complete** combined dispatcher, `place_configured_feature_all`: M5-B07's original 7 kinds (delegated verbatim) plus all 35 of this family's own kinds (M5-B12a's 5 + M5-B12b's 9 + M5-B12c's 12 + M5-B12d's 2 + this blueprint's own 7) plus the 4 End-dimension-only kinds (a documented no-op arm) plus a final catch-all no-op for every name not yet registered anywhere (M5-B11's still-unimplemented vegetation kinds, at this blueprint's own point in the dependency order). `place_configured_feature_all` carries a `ctx: &PlacementCtx` parameter **from its very first draft** — the original, unsplit `M5-B12-features-underground-misc.md` omitted this, which meant this blueprint's own 6 delegating kinds (5 selectors + `sequence`) could not actually be implemented as specified (each needs to re-enter `run_placement_chain` via a `ctx` its own `delegate` helper had no way to obtain). Splitting the family into five blueprints, with this one landing last, closes that gap structurally: `ctx` is simply part of the signature from the start, threaded straight through from `driver.rs`'s own already-constructed `PlacementCtx` (M5-B07 Context §E.3) to every delegated re-entrant call.

Done when:

- [ ] `cargo build -p rc-worldgen` succeeds with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-worldgen`.
- [ ] The dispatcher registration table, `sequence`/`no_op`/the four selector kinds' draw-count-and-order proofs, and `scattered_ore`'s drawn-try-count-and-jitter proof reproduce their stated exact values/counts exactly (all HIGH confidence).
- [ ] The full-registry coverage audit accounts for every one of the 64 `Feature` kind names and 14 remaining trunk/foliage placer kinds M5-B07 §M/§N.5 enumerates, with no name appearing in neither an "implemented" nor a "deferred-with-owner" column — restated once, completely, in `blueprints/M5/M5-B00-index.md`, not duplicated here.
- [ ] `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps` all exit 0 (no new dependency edges).
- [ ] `cargo test --doc -p rc-worldgen` exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### 0. Prerequisite API recap

- **`WorldgenRandom<AnyRandom>`** — `random.next_int_bounded(n)`, `.next_bool()` — M5-B01's own exact algorithms. This blueprint never constructs a fresh `WorldgenRandom` itself — every kind receives the SAME already-seeded carrier the driver passes, exactly as M5-B07 documents ("the RNG stream simply continues across every one of that feature's own multiple placement attempts within one chunk").
- **`DecorationWorldAccess`/`BlockStateResolver`/`BlockPropertyResolver`** (M5-B07) — identical shape to every other family member.
- **`crate::decoration::modifiers::{run_placement_chain, PlacementCtx}`** (M5-B07 Deliverables, unchanged):
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
The re-entrant call this blueprint's own `delegate` helper makes (§K below) constructs a **new** `PlacementCtx` identical to the caller's own except `feature_name`, which is rebound to the nested feature's own `ResourceLocation`. This is a deliberate, documented deviation from vanilla: vanilla's own nested-delegation path never resolves a `Biome{}` placement modifier at all — its re-entrant call leaves the top-feature slot unset, so a `Biome{}` modifier reached through a nested chain would raise vanilla's own unregistered-feature error, and vanilla's own data never places a `Biome{}` modifier inside a nested chain in the first place, only at the top level, where it is the OUTER placed feature's own identity that gets checked. Rebinding `feature_name` to the nested feature here is strictly more permissive than vanilla (it lets a nested `Biome{}` modifier resolve instead of erroring) and is kept as this project's own error-avoiding convention rather than a reproduction of vanilla's throw.
- **`crate::data::{WorldgenData, ConfiguredFeature, ResourceLocation}`** (M5-B02) — `WorldgenData { configured_features, placed_features, .. }`; a missing `placed_features`/`configured_features` lookup is a loud `panic!`, matching M5-B07's own data-integrity stance.
- **`crate::decoration::features::ore::place`** (M5-B07) — the vanilla `ore` kind's own full blob-placement algorithm; `scattered_ore` does **not** reuse this function. It shares only a target-rule-plus-air-exposure admissibility check with the `ore` kind, which M5-B07's own module does not yet separately expose as a callable helper — a gap for the planning role to close, since this blueprint cannot add a public function to another blueprint's already-specified file.
- **M5-B07's original `place_configured_feature`** (`crate::decoration::features::place_configured_feature`) — dispatches only its own 7 original kinds (`ore`, `disk`, `spring_feature`, `lake`, `tree`, `random_patch`, `simple_block`); untouched, still independently correct and independently tested.
- Every family sibling's own `place_*` function (M5-B12a `dripstone`/`geode`/`sculk`, M5-B12b `nether`/`ice`, M5-B12c `misc`, M5-B12d `structure_bridge`) is imported and called directly by this blueprint's own `place_configured_feature_all` body — the full list is Table K.1 below.

### A. Scope

This blueprint owns: `no_op`, `random_selector`, `weighted_random_selector`, `simple_random_selector`, `random_boolean_selector`, `sequence`, `scattered_ore` (7 kinds), plus the family's own combined-dispatcher assembly. See `blueprints/M5/M5-B00-index.md` for the full 64-kind coverage identity across M5-B07 (7) + M5-B12a-e (35) + M5-B11 (17) + End-exclusive (5).

### K. Utility / meta-combinators — `no_op`, `random_selector`, `weighted_random_selector`, `simple_random_selector`, `random_boolean_selector`, `sequence`, `scattered_ore`

Every kind in this section (except `scattered_ore`) delegates to ANOTHER named `placed_feature` via a full, re-entrant `run_placement_chain` call at the SAME origin, on the SAME shared RNG stream — identical in spirit to M5-B07's own `random_patch`. All six need `data: &WorldgenData` (to resolve the nested `placed_feature` name), `ctx: &PlacementCtx` (to re-enter `run_placement_chain`), and `bridge: Option<&UndergroundFeatureContext>` (so a nested feature reachable through a selector can itself be `fossil`/`template`) threaded through —**this is the exact defect the original, unsplit `M5-B12-features-underground-misc.md` had**: its own combined dispatcher's public signature omitted `ctx`, so these six kinds could not actually be implemented as specified. This blueprint's own `place_configured_feature_all` (§S below) carries `ctx` from the start; the gap does not exist here.

**K.1 — `no_op` (HIGH confidence — a genuine, registered, zero-effect kind, distinct from "unrecognized name," M5-B07's own no-op-by-default policy).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct NoOpConfiguration {}
```
`fn place_no_op(_config, _origin, _world, _resolver, _props, _random) -> bool { true }` — zero draws, zero writes, always succeeds.

**K.2 — `random_selector` (HIGH confidence — sequential probability-gated selection with a default fallback; first success wins).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct WeightedPlacedFeatureRef { pub feature: crate::data::ResourceLocation, pub chance: f32 }
#[derive(serde::Deserialize, Debug, Clone)]
pub struct RandomSelectorConfiguration { pub features: Vec<WeightedPlacedFeatureRef>, pub default: crate::data::ResourceLocation }
```

```text
fn place_random_selector(origin, config, world, resolver, props, random, data, ctx, bridge) -> bool:
    for entry in &config.features:
        if random.next_float() < entry.chance:                       // 1 draw PER entry checked, stops at first success
            return delegate(&entry.feature, origin, world, ctx, random, data, resolver, props, bridge)
    delegate(&config.default, origin, world, ctx, random, data, resolver, props, bridge)   // zero further draws
```

**K.3 — `weighted_random_selector` (HIGH confidence — cumulative-weight selection, the same shape as M5-B07's `HeightProvider::WeightedList`; this kind has no default feature at all).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct WeightedEntry { pub data: crate::data::ResourceLocation, pub weight: u32 }
#[derive(serde::Deserialize, Debug, Clone)]
pub struct WeightedRandomSelectorConfiguration { pub features: Vec<WeightedEntry> }
```

```text
fn place_weighted_random_selector(origin, config, world, resolver, props, random, data, ctx, bridge) -> bool:
    total: u32 = config.features.iter().map(|e| e.weight).sum()
    if total == 0: return true                                           // zero draws, nothing placed — no default exists on this kind
    mut roll = random.next_int_bounded(total as i32) as u32              // ONE draw
    for entry in &config.features:
        if roll < entry.weight: return delegate(&entry.data, ...)
        roll -= entry.weight
    unreachable!()                                                       // every draw < total, so some entry always satisfies roll < entry.weight
```

**K.4 — `simple_random_selector` (HIGH confidence — uniform pick among N, one draw).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct SimpleRandomSelectorConfiguration { pub features: Vec<crate::data::ResourceLocation> }
```
`idx = random.next_int_bounded(config.features.len() as i32)` (ONE draw) → delegate to `config.features[idx]`.

**K.5 — `random_boolean_selector` (HIGH confidence — a fair coin flip between two named features).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct RandomBooleanSelectorConfiguration { pub feature_true: crate::data::ResourceLocation, pub feature_false: crate::data::ResourceLocation }
```
`if random.next_bool() { delegate(&config.feature_true, ...) } else { delegate(&config.feature_false, ...) }` — ONE draw.

**K.6 — `sequence` (HIGH confidence — runs its listed features in list order at the SAME origin, each to full completion before the next begins, but stops the moment one of them fails: the first delegated feature whose own placement does not succeed aborts the sequence and every later entry is skipped entirely — zero RNG of its own beyond whatever each delegated feature's own chain consumes).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct SequenceConfiguration { pub features: Vec<crate::data::ResourceLocation> }
```

```text
fn place_sequence(origin, config, world, resolver, props, random, data, ctx, bridge) -> bool:
    for name in &config.features:
        if !delegate(name, origin, world, ctx, random, data, resolver, props, bridge):
            return false                                                 // first failed delegate aborts; every later entry is skipped entirely
    true
```

**K.7 — `scattered_ore` (HIGH confidence — draws its own repetition count, jitters each attempt's target position along all three axes from a shared falloff formula, and places at most one block per attempt through the same admissibility check the vanilla `ore` kind uses — never the `ore` kind's own blob-placement algorithm).**

```rust
#[derive(serde::Deserialize, Debug, Clone)]
pub struct ScatteredOreConfiguration { pub inner: crate::decoration::features::OreConfiguration }
```

```text
fn place_scattered_ore(origin, config, world, resolver, props, random) -> bool:
    tries = random.next_int_bounded(config.inner.size + 1)               // ONE draw, taken once, before any attempt; size is OreConfiguration's own 0..=64 field; zero tries is a normal outcome
    for i in 0..tries:
        max_dist = min(i, 7)                                             // attempt 0 always lands exactly on origin
        dx = round((random.next_float() - random.next_float()) * max_dist)   // 2 draws
        dy = round((random.next_float() - random.next_float()) * max_dist)   // 2 draws
        dz = round((random.next_float() - random.next_float()) * max_dist)   // 2 draws — x, then y, then z: 6 next_float draws per attempt
        target = origin + (dx, dy, dz)
        state = world.get_block(target)
        for target_state in &config.inner.target_states:                 // first passing entry wins, at most one block written
            if ore::can_place_target(target_state, state, config.inner.discard_chance_on_air_exposure, world, resolver, random, target):
                world.set_block(target, target_state.state)              // this attempt's only write
                break
    true                                                                  // always succeeds, even with zero tries or nothing written
```
`place_scattered_ore` never calls `ore::place` (M5-B07's own full blob-placement algorithm — a materially different shape: one `next_float` angle, two `next_int_bounded(3)` Y-endpoint draws, a lerped double-ended ellipsoid blob sampled `config.inner.size` times). Only the shared target-rule-plus-air-exposure admissibility check, `ore::can_place_target` above, is reused; that helper is not yet exposed by M5-B07's own module (Context §0), which is an open gap for the planning role to close rather than a settled GEN-D16 resolution.

`delegate(name, origin, world, ctx, random, data, resolver, props, bridge) -> bool` (an internal helper every kind in this section except `scattered_ore` shares): looks up `placed = &data.placed_features[name]` (a missing entry is a loud `panic!`), then calls `run_placement_chain(&placed.placement, origin, world, ctx, random, &mut |w,p,r| place_configured_feature_all(&data.configured_features[&placed.feature], p, w, resolver, props, r, data, ctx, bridge))` — a genuine nested full-placement-modifier-chain call, exactly M5-B07's own `random_patch` shape, generalized to route back through THIS blueprint's own combined dispatcher (§S) rather than M5-B07's narrower one, so a selector/`sequence` member reachable this way can itself be any of this family's 35 kinds, not only M5-B07's original 7. The nested `PlacementCtx` `run_placement_chain` receives internally rebinds `feature_name` to `name` (Context §0's own documented deviation from vanilla). `delegate` returns whatever `place_configured_feature_all` itself returns for the nested feature (§S).

### N. End-dimension-only kinds — deferred, no owner yet reserved

`end_platform`, `end_spike`, `end_island`, `end_gateway` are structurally unreachable through the driver's own `decorate_chunk` (no per-biome feature data is compiled for the End dimension anywhere in this project) — implementing their algorithms would be dead code with no acceptance-test path. `place_configured_feature_all` (§S) still names all 4 explicitly as a documented no-op arm, distinct from "unrecognized name," so a future End-dimension-support blueprint has an exact, named seam to fill in.

### S. The combined dispatcher — `place_configured_feature_all`

```rust
/// Supersedes M5-B07's own `place_configured_feature` as `decorate_chunk`'s call site
/// (below); that function itself is untouched and remains directly callable (and directly
/// unit-tested) for its own 7 kinds. Dispatches: M5-B07's 7 kinds (delegated verbatim),
/// M5-B12a's 5 kinds, M5-B12b's 9 kinds, M5-B12c's 12 kinds, M5-B12d's 2 kinds, this
/// blueprint's own 7 kinds, the 4 End-dimension kinds (Context §N, an explicit no-op arm),
/// and a final catch-all no-op for every name not yet registered anywhere (M5-B11's own
/// still-unimplemented vegetation kinds, at this blueprint's own point in time) —
/// identical `debug`-logged-no-op policy to M5-B07. `ctx: &PlacementCtx` is REQUIRED (the
/// fix this blueprint's own position in the family makes structurally unavoidable to skip):
/// every one of this blueprint's own 6 delegating kinds needs it to re-enter
/// `run_placement_chain`. Returns `bool` so `sequence`'s own short-circuit (Context §K.6)
/// can observe a nested failure: for this blueprint's own 7 kinds the return value is the
/// kind's own real, computed success; for the 42 kinds owned by M5-B07 and M5-B12a/b/c/d,
/// none of which report success themselves today, this dispatcher conservatively returns
/// `true` regardless of what actually happened — a bounded, documented deviation tracked
/// for the planning role, not a claim that those kinds cannot fail.
#[allow(clippy::too_many_arguments)]
pub fn place_configured_feature_all(
    feature: &crate::data::ConfiguredFeature,
    origin: rc_core::BlockPos,
    world: &mut dyn super::context::DecorationWorldAccess,
    resolver: &dyn super::context::BlockStateResolver,
    props: &dyn super::context::BlockPropertyResolver,
    random: &mut crate::random::WorldgenRandom<crate::noise::AnyRandom>,
    data: &crate::data::WorldgenData,
    ctx: &crate::decoration::modifiers::PlacementCtx,
    bridge: Option<&UndergroundFeatureContext>,
) -> bool;
```

**One-line additive change to M5-B07's already-specified `driver.rs`:** `decorate_chunk`'s own per-feature call site (M5-B07 Context §E.3, step 3's last bullet) changes from `place_configured_feature(&data.configured_features[&placed.feature], pos, world, resolver, props, random, data)` to `place_configured_feature_all(&data.configured_features[&placed.feature], pos, world, resolver, props, random, data, &ctx, bridge)`, where `ctx` is the SAME `PlacementCtx` `decorate_chunk` already constructs per feature (M5-B07 Context §E.3 — this blueprint adds no new construction, only forwards the existing one one layer further). `decorate_chunk`'s own signature gains one trailing parameter, `bridge: Option<&UndergroundFeatureContext>`, threaded through from its own caller (a future `GenStage`-driver blueprint's own responsibility to supply a real `UndergroundFeatureContext` once a real `DirectoryTemplateSource`/`BlockStateNames` registry exists — `None` until then, exactly M5-B08's own beardifier seam's identical bootstrap story; M5-B09's own text needs a small, additive correction to pass `bridge: None` at its own `advance_to_features` call site, tracked in `blueprints/M5/M5-B00-index.md`'s "Cross-blueprint gaps and reconciliation" — out of this blueprint's own assigned file scope). No other line of M5-B07's own Deliverables changes.

**Table K.1 — the full dispatch table this blueprint's own `place_configured_feature_all` body implements** (every entry below is a direct, unmodified call to a sibling blueprint's own already-shipped `place`/`place_*` function; no algorithm is re-derived):

| `feature_type` | Delegates to |
|---|---|
| `ore`, `disk`, `spring_feature`, `lake`, `tree`, `random_patch`, `simple_block` | `crate::decoration::features::place_configured_feature`'s own internal per-kind functions (M5-B07, verbatim) |
| `large_dripstone`, `speleothem`, `speleothem_cluster` | `super::dripstone::place` (M5-B12a) |
| `geode` | `super::geode::place` (M5-B12a) |
| `sculk_patch` | `super::sculk::place` (M5-B12a) |
| `delta_feature`, `basalt_columns`, `basalt_pillar`, `netherrack_replace_blobs`, `glowstone_blob` | `super::nether::place` (M5-B12b) |
| `iceberg`, `blue_ice`, `spike` | `super::ice::place` (M5-B12b) |
| `freeze_top_layer` | `super::ice::place` with `freeze: bridge.map(|b| b.freeze)` — a documented no-op when `bridge.is_none()` (M5-B12b/M5-B12d) |
| `root_system`, `multiface_growth`, `underwater_magma`, `monster_room`, `block_pile`, `block_column`, `replace_single_block`, `block_blob`, `desert_well`, `void_start_platform`, `fill_layer`, `bonus_chest` | `super::misc::place` (M5-B12c) |
| `fossil` | `super::structure_bridge::place_fossil` (M5-B12d) |
| `template` | `super::structure_bridge::place_template` (M5-B12d) |
| `no_op` | `place_no_op` (this blueprint) |
| `random_selector`, `weighted_random_selector`, `simple_random_selector`, `random_boolean_selector`, `sequence` | `place_random_selector`/`place_weighted_random_selector`/`place_simple_random_selector`/`place_random_boolean_selector`/`place_sequence` (this blueprint) |
| `scattered_ore` | `place_scattered_ore` (this blueprint) |
| `end_platform`, `end_spike`, `end_island`, `end_gateway` | explicit no-op arm (Context §N) |
| anything else | catch-all `debug`-logged no-op (M5-B11's own still-unimplemented kinds land here until M5-B11 lands) |

### Porting-pitfall checklist (this blueprint's own additions)

1. **`beehive`/`cocoa`-style conditional draws are not this blueprint's own concern** (that is M5-B11's), but the identical discipline applies here: `random_selector`'s per-entry `chance` roll stops at the FIRST success — entries after the first success consume zero further draws.
2. **`sequence` runs each listed feature to full completion, in list order, at the SAME origin, but stops at the first one that fails** — every later entry is then skipped entirely; among entries that DO run, a later one can overwrite an earlier one's own blocks, which is correct and matches vanilla's own layered-sequence behavior, never a bug to "fix" by skipping already-written positions.
3. **`delegate`'s re-entrant `PlacementCtx` rebinds ONLY `feature_name`** — every other field (`step`, `biome_defs`, `biome_names`, `resolver`, `props`) is copied unchanged from the caller's own `ctx`. This rebinding is this project's own deliberate deviation from vanilla, which never resolves a biome check inside a nested chain at all (Context §0).
4. **This is the ONLY blueprint in the M5-B12 family whose own `place_configured_feature_all` body actually exists** — M5-B12a/b/c/d's own kind functions are real and independently tested, but the combined dispatcher that reaches them by name is finalized here, last, by design (Context §0's own restated rationale).

### Claims to verify (TEST-D57)

- no_op is a genuine registered Feature kind, distinct from an unrecognized name, that performs zero draws and zero writes.
- random_selector evaluates its listed entries in order, drawing one next_float per entry checked, and selects the first entry whose draw is less than that entry's chance value, consuming zero further draws once a success is found.
- random_selector falls through to its default feature, with zero further draws, if none of its listed entries succeed.
- weighted_random_selector sums the weight field of every listed entry into a total; this kind has no default feature at all, so when that total is 0 it selects nothing and consumes zero draws.
- weighted_random_selector, when the total weight is nonzero, draws exactly one next_int_bounded(total) value, then walks its entries in list order subtracting each entry's weight from the running roll until the entry where roll is less than that entry's weight is selected.
- simple_random_selector draws exactly one next_int_bounded(N) value, where N is the number of listed features, and selects the feature at that index.
- random_boolean_selector draws exactly one next_bool() value, selecting feature_true on true and feature_false on false.
- sequence runs its listed features in list order at the same origin, each to full completion before the next begins, but stops the moment one delegated feature fails: the first failure aborts the sequence and every later entry is skipped, consuming zero RNG draws of its own beyond whatever each delegated feature's own chain consumes.
- In sequence, later entries in the list can overwrite earlier entries' own blocks, which is correct and matches vanilla's own layered-sequence behavior.
- scattered_ore draws one next_int_bounded(size + 1) value before any attempt, where size is OreConfiguration's own field, and repeats its placement attempt that many times (zero attempts is a normal outcome).
- Each of scattered_ore's attempts computes dx as round((next_float() - next_float()) * min(attempt_index, 7)) — two next_float draws, not a bounded-int draw.
- Each of scattered_ore's attempts computes dz the same way as dx — round((next_float() - next_float()) * min(attempt_index, 7)) — and also draws dy by the identical formula between the x and z draws, applying all three offsets; the Y coordinate is not left unchanged.
- scattered_ore does not reuse the ore feature kind's own placement algorithm; each attempt reads the block state at the jittered position offset by (dx, dy, dz) and, for the first target-state entry passing the shared admissibility check the ore kind also uses, writes a single block there.
- The RNG stream used by a feature's placement continues across every one of that feature's own multiple placement attempts within one chunk, rather than being reseeded between attempts.
- Vanilla never resolves a Biome placement modifier inside a nested selector/sequence delegation chain at all: the nested-call path leaves the top feature unset, and vanilla's own data never places a Biome modifier inside a nested chain; only the top-level call binds a checkable feature, and it is the outer placed feature's own identity that gets checked there.
- random_selector, weighted_random_selector, simple_random_selector, and random_boolean_selector each place their selected feature at the same block position as the outer call, continuing the same already-seeded RNG stream rather than starting a new one.
- end_platform, end_spike, end_island, and end_gateway are Feature kinds that vanilla only ever places in the End dimension.

## Deliverables

### `crates/worldgen/src/decoration/underground/selectors.rs` (NEW)

`NoOpConfiguration`, `WeightedPlacedFeatureRef`, `RandomSelectorConfiguration`, `WeightedEntry` (its `data` field, not `feature`), `WeightedRandomSelectorConfiguration` (no `default` field — this kind has none), `SimpleRandomSelectorConfiguration`, `RandomBooleanSelectorConfiguration`, `SequenceConfiguration`, `ScatteredOreConfiguration` plus one `place`/`place_*` fn each, each returning `bool` (each of the six delegating kinds takes the additional `data: &WorldgenData, ctx: &PlacementCtx, bridge: Option<&UndergroundFeatureContext>` parameters their own re-entrant `run_placement_chain`/`place_configured_feature_all` calls need, and `delegate`/`place_sequence` propagate that `bool` so `sequence`'s own short-circuit (Context §K.6) can observe a nested failure), exactly per Context §K. Also defines the shared internal `delegate(...) -> bool` helper (Context §K's own closing paragraph).

### `crates/worldgen/src/decoration/underground/mod.rs` (MODIFY — final change to this file; one new module line PLUS the full `place_configured_feature_all` body)

```rust
pub mod selectors;

#[allow(clippy::too_many_arguments)]
pub fn place_configured_feature_all(
    feature: &crate::data::ConfiguredFeature,
    origin: rc_core::BlockPos,
    world: &mut dyn super::context::DecorationWorldAccess,
    resolver: &dyn super::context::BlockStateResolver,
    props: &dyn super::context::BlockPropertyResolver,
    random: &mut crate::random::WorldgenRandom<crate::noise::AnyRandom>,
    data: &crate::data::WorldgenData,
    ctx: &crate::decoration::modifiers::PlacementCtx,
    bridge: Option<&UndergroundFeatureContext>,
) -> bool {
    // body: Table K.1's full match on `feature.feature_type`, exactly per Context §S.
}
```

### `crates/worldgen/src/decoration/driver.rs` (MODIFY — Context §S's additive extension)

`decorate_chunk`'s signature gains one trailing parameter `bridge: Option<&UndergroundFeatureContext>`; its per-feature call site swaps `place_configured_feature(...)` for `crate::decoration::underground::place_configured_feature_all(..., &ctx, bridge)`, forwarding the `PlacementCtx` `decorate_chunk` already constructs. No other line changes — every other step of M5-B07's algorithm (seeding, possible-biomes computation, per-step reachable-set union, global-index sort) is untouched.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary** (TEST-D45/D46): every new file above is committed with every function body `todo!()`-stubbed in this first changeset, alongside every test file below. The implementation changeset fills bodies only. This blueprint's own changeset does **not** touch `decoration/mod.rs`/`driver.rs` in the test-first commit (those additive changes land in the implementation changeset, since `decorate_chunk`'s new parameter is exercised only via `underground_dispatcher_registration.rs`, which constructs its own `PlacementCtx`/driver call directly).

### `crates/worldgen/tests/underground_selectors_and_no_op.rs`

1. `no_op_zero_draws_zero_writes` — any config, any world state; `place_no_op` leaves both untouched.
2. `random_selector_stops_at_first_success` — 3 entries with `chance=[0.0, 1.0, 1.0]`; exactly 2 `next_float` draws consumed (entry 1's failing check, entry 2's succeeding check) and the delegated feature is entry 2's, never entry 3's.
3. `weighted_random_selector_cumulative_selection` — 3 entries with weights `[1,1,8]`; a fixed seed whose single `next_int_bounded(10)` draw is hand-verified `>= 2`; the THIRD entry is selected.
4. `simple_random_selector_uniform_one_draw` — 4 entries; exactly one `next_int_bounded(4)` draw consumed, selecting `features[draw_result]`.
5. `random_boolean_selector_one_draw_two_outcomes` — a fixed seed whose `next_bool()` is hand-verified `true`; `feature_true` is selected, exactly one draw.
6. `sequence_runs_every_entry_at_same_origin_in_order` — 3 nested `simple_block`-configured features (M5-B07's own kind), each with a DISTINCT state, each succeeding (a stubbed `delegate` returning `true` for all 3); all 3 end up written at `origin` in list order (the LAST one's state is what `world.get_block(origin)` ultimately reads), and `place_sequence` itself returns `true`. A companion case with the first delegate call stubbed to return `false` asserts `place_sequence` returns `false` and the remaining entries are never delegated to.
7. `scattered_ore_draws_a_bounded_try_count_and_never_calls_ore_place` — a fixed seed whose single `next_int_bounded(size + 1)` draw is hand-verified to a known try count N; exactly N attempts run, each preceded by exactly 6 fresh `next_float` draws (the XYZ jitter) and at most one `world.set_block` call; an instrumented wrapper around M5-B07's own `ore::place` asserts it is never invoked; `place_scattered_ore` itself always returns `true`.
8. **`random_selector_and_sequence_reachable_through_the_real_combined_dispatcher`** — calls `place_configured_feature_all` directly (not `place_random_selector`/`place_sequence` as unit calls) with a `feature_type: "minecraft:random_selector"` config whose nested `entry.feature` resolves to a `simple_block`-configured `placed_feature`, and separately with `feature_type: "minecraft:sequence"` referencing two nested features; asserts the delegated block(s) are actually written at `origin` through the FULL `place_configured_feature_all(..., ctx, bridge)` entry point, proving `ctx` is correctly threaded from the dispatcher's own parameter into `delegate`'s re-entrant `run_placement_chain` call — this is the exact class of gap the original, unsplit `M5-B12-features-underground-misc.md` left untested (its own `place_random_selector` unit tests never exercised the combined dispatcher's own signature at all).

### `crates/worldgen/tests/underground_dispatcher_registration.rs`

1. `dispatches_m5b07_kinds_to_their_own_functions` — a `ConfiguredFeature{feature_type: "minecraft:simple_block", ..}`; `place_configured_feature_all` produces the identical world state M5-B07's own `place_configured_feature` would for the same input.
2. `dispatches_all_35_family_kinds_by_name` — a table-driven test: for each of the 35 kind names in Table K.1, a minimal valid synthetic config, asserting `place_configured_feature_all` does not panic and (for the zero-RNG deterministic kinds — `no_op`, `sequence` with an empty list, `desert_well`, `fill_layer`, `replace_single_block` with no matching target) produces the exact documented no-op/deterministic result.
3. `end_dimension_kinds_are_a_documented_no_op_not_unrecognized` — `feature_type: "minecraft:end_gateway"` (and the other 3, table-driven); zero writes, zero draws, and the dispatch reaches an EXPLICIT End-dimension arm, not the generic unrecognized-name catch-all (satisfied by code inspection of the dispatch arm existing, acceptable per this test's own structural nature).
4. `unrecognized_name_falls_through_to_documented_no_op` — `feature_type: "minecraft:some_future_vegetation_kind"` (one of M5-B11's still-unimplemented names); zero writes, zero draws, no panic.
5. `freeze_top_layer_no_ops_when_bridge_absent` — `feature_type: "minecraft:freeze_top_layer"`, `bridge: None`; zero writes, zero draws (Table K.1's own documented `bridge.map(...)` no-op path).

## Implementation steps

1. **`decoration/underground/selectors.rs`.** All 7 kinds + the shared `delegate` helper, exactly per Context §K. Observable: `underground_selectors_and_no_op.rs`'s per-kind unit tests pass (test 8 still fails — the combined dispatcher does not exist yet).
2. **`decoration/underground/mod.rs` (`place_configured_feature_all` body).** The full dispatch table (Table K.1): M5-B07's 7 kinds, M5-B12a's 5, M5-B12b's 9, M5-B12c's 12, M5-B12d's 2, this blueprint's own 7, the 4 End-dimension no-op arms, the final catch-all no-op. Observable: `underground_selectors_and_no_op.rs` test 8 and `underground_dispatcher_registration.rs` both pass.
3. **`decoration/driver.rs` (Context §S's additive extension).** `decorate_chunk`'s signature gains `bridge`; its one call site swaps to `place_configured_feature_all(..., &ctx, bridge)`. Observable: `cargo build -p rc-worldgen` succeeds with zero `todo!()` remaining anywhere in the M5-B12 family's own files; M5-B07's own full existing test suite still passes unmodified.
4. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` all exit 0.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding** — every test file above is committed first, verbatim, alongside `todo!()`-stubbed source files; the implementation changeset fills bodies only. M5-B12a/b/c/d's own already-specified test files are never touched by this blueprint at all, in either changeset.

(b) **No new `[workspace.dependencies]` entry and no new `Cargo.toml` line.**

(c) **No Mojang or third-party reimplementation source is consulted.** Every algorithm in this blueprint's own Context section is either (i) restated in full from an earlier, already-derived blueprint, or (ii) this blueprint's own honest reconstruction from public documentation, at the confidence level explicitly stated.

(d) **M5-B07's own already-specified files (`features/mod.rs`, `driver.rs`'s pre-existing algorithm body) and M5-B12a/b/c/d's own already-specified files are never rewritten** — this blueprint's only touch to M5-B07's own Deliverables is the single, explicitly-scoped additive extension named in Context §S (one new trailing parameter on `decorate_chunk`, one call-site swap). M5-B07's own `place_configured_feature` function body is untouched and remains independently correct and independently tested for its own 7 kinds.

(e) **Gen-time block writes never call, or route through, `01`'s tick-time update engine.** No dependency edge from `rc-worldgen` to `rc-mechanics` is added.

(f) **No light-engine call of any kind.**

(g) **GEN-D20's tie-break and this blueprint's own confidence-tier flags must never be conflated.**

(h) **Every algorithm's own stated RNG draw order must never be "cleaned up" or reordered** for readability or performance, even where marked low confidence — a low-confidence algorithm must still be exactly, deterministically reproducible run-to-run.

(i) **No `unsafe` code.**

## Verification commands

- `cargo build -p rc-worldgen` — zero warnings.
- `cargo nextest run -p rc-worldgen` — every test in `underground_selectors_and_no_op.rs`, `underground_dispatcher_registration.rs` passes, AND M5-B07's own pre-existing `decoration_*.rs` test suite and M5-B12a/b/c/d's own test suites all still pass unmodified.
- `cargo test --doc -p rc-worldgen` — exits 0.
- `cargo run -p xtask -- fmt-check` / `-- lint` / `-- lint-deps` — all exit 0.
- CI tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D37/D50).
