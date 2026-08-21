# M5-B08 — Structures: Placement, Jigsaw Assembly, Templates & Persistence

| Field | Content |
|---|---|
| ID | M5-B08 |
| Milestone | M5 — World Generation Parity |
| Prerequisites | M5-B01 (RNG core: `RcLegacyRandom`/`RcXoroshiroRandom`, `BitSource`/`RcRandomSource`, `WorldgenRandom<B>`, `LegacyPositionalFactory`/`XoroshiroPositionalFactory`, `set_large_feature_seed`/`set_large_feature_with_salt` — this blueprint consumes these exactly, never re-derives a formula); M5-B02 (worldgen data pipeline: the compiled `rc_worldgen::data` types this blueprint reads — `StructureSet`/`StructurePlacement`/`StructureSelectionEntry`, `Structure`/`StructureId`, `TemplatePool`/`PoolElement`/`Projection`, `ProcessorList`/`StructureProcessor`/`RuleTest`/`PosRuleTest`, `BlockStateSpec`, `DecorationStep`, `TerrainAdaptation`, `WorldgenData`); M5-B03 (density interpreter: `EvalContext`, `DensityInterpreter`, `NoiseChunk`, `evaluate_node`'s `Beardifier{}` arm — currently a fixed `0.0` stub this blueprint replaces with a real contribution); M5-B05 (biomes: `ClimateSampler`, `MultiNoiseBiomeSource`, `TargetPoint` — this blueprint's structure biome checks and `ConcentricRingsPlacement`'s ring biome search sample through these, never re-deriving biome lookup). |
| Implements | GEN-D6 (large-feature and large-feature-with-salt seed formulas — restated only as call sites into M5-B01's own functions, never re-derived), GEN-D21 (structures are a pure function of `(world seed, structure-set grid cell, noise-router biome/height samples)` — this blueprint's central architectural claim, realized concretely), GEN-D22 (region/cluster border structure generation requires zero messaging — restated and shown to hold for every algorithm this blueprint defines), GEN-D23 (structure NBT templates never committed; operator-supplied runtime loading — this blueprint's `template::DirectoryTemplateSource` is the concrete mechanism), GEN-D25 (structure-starts/structure-references as the first two `GenStage` rungs, and this blueprint's output as the payload a future GenStage-integration blueprint delivers via the Stage-1 structural command). |
| Crates touched | `rc-worldgen` (`crates/worldgen/`): new `src/structure/` module tree (`mod.rs`, `placement.rs`, `generation.rs`, `jigsaw.rs`, `template.rs`, `processor.rs`, `beardifier.rs`, `persistence.rs`); `Cargo.toml` (add `rc-nbt = { path = "../nbt" }` — already a sibling crate in this workspace, M2-B02, not a new external dependency); `src/lib.rs` (add `pub mod structure;`); `src/density/interpreter.rs` and `src/density/noise_chunk.rs` (M5-B03's own files — extend `evaluate_node`'s signature and both `DensityInterpreter`/`NoiseChunk` structs with an optional beardifier context, replacing the fixed `0.0` stub). `rc-chunk-storage` (`crates/chunk-storage/`): `src/chunk_nbt.rs` only (M2-B04's own file — replace the fixed-empty `structures` placeholder with a real, opaque passthrough field; no `Cargo.toml` change). |
| Estimated scope | L (large even by M5's own precedent — placement, jigsaw assembly, template/processor handling, beardifier wiring, and cross-crate persistence in one coherent domain; matches M5-B02/M5-B03's own >1000-line scope, not the general ≤800-line guideline, per this milestone's established practice for interpreter-and-assembly-heavy blueprints). |

## Goal & Done definition

Give `rc-worldgen` vanilla's structure-generation domain (GEN-D21–D23): structure-set placement (`random_spread` grid+jitter with all four frequency-reduction hash variants and exclusion zones; `concentric_rings`' once-per-world ring-position algorithm), the structure-starts/structure-references two-phase generation flow with biome-check semantics and weighted multi-structure selection, the complete jigsaw assembly algorithm (template pools, pool aliases, the priority-bucketed BFS placer, the asymmetric jigsaw-attach rule, rotation/mirror handling with the exact RNG draw order, collision via per-branch/ambient free space, terrain adaptation and the beardifier density contribution wired into M5-B03's interpreter), operator-supplied structure NBT template loading and the full 11-processor pipeline, and structure-start/reference chunk-NBT persistence. Every non-jigsaw (hand-coded) structure family is named and explicitly deferred to M5-B12 (reserved, not yet drafted, Context §A) — the research corpus does not document those 15 families' piece-generation grammars precisely enough to restate bit-exactly here.

Done when:

- [ ] `cargo build -p rc-worldgen` and `cargo build -p rc-chunk-storage` both succeed with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p rc-worldgen` and `cargo nextest run -p rc-chunk-storage`.
- [ ] Every placement-grid known-answer test reproduces this blueprint's own hand-derived vector exactly (bit-for-bit `i32`/`i64` equality — Context §B's vectors were computed by this blueprint's own derivation pass executing the exact §B/§C algorithms in Python with faithful Java 32-/64-bit wrapping semantics, the same methodology M5-B01's own research source uses).
- [ ] Every jigsaw assembly golden-run test reproduces its expected piece count, position, and rotation exactly, using only hand-authored, own-made template fixtures (Constraints (c)) constructed so the outcome does not depend on the exact RNG bit stream (Acceptance tests' own stated rationale).
- [ ] `cargo run -p xtask -- lint-deps` still exits 0 — `rc-worldgen`'s one new dependency edge (`rc-nbt`) is an existing internal crate already in this workspace's dependency graph (M2-B02), not a new external dependency or a new `SIM`/`NETRENDER` edge.
- [ ] `cargo run -p xtask -- fmt-check` and `-- lint` both exit 0.
- [ ] `cargo test --doc -p rc-worldgen` and `cargo test --doc -p rc-chunk-storage` both exit 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37), on a clean checkout (TEST-D50).

## Context (self-contained)

### A. Scope tiering — what this blueprint implements vs. names and defers

Vanilla 26.2's `StructureType` registry has 16 entries (`docs/research/mc-26.2/06-structures.md` §3.1): `jigsaw`, plus 15 hand-coded families each with dedicated Java piece-generation code (`buried_treasure`, `desert_pyramid`, `end_city`, `fortress` [nether fortress], `igloo`, `jungle_temple`, `mineshaft`, `nether_fossil`, `ocean_monument`, `ocean_ruin`, `ruined_portal`, `shipwreck`, `stronghold`, `swamp_hut`, `woodland_mansion`). The vast majority of *actual placed content* — every village variant, pillager outposts, bastion remnants, ancient city, trail ruins, trial chambers — is `jigsaw`-typed and needs **zero** structure-family-specific Rust code beyond this blueprint's own generic jigsaw engine (Context §F); only datapack configuration differs per structure. This blueprint therefore implements, completely:

- Structure-set placement for **every** structure family regardless of kind (`random_spread` and `concentric_rings` are placement-layer concerns, orthogonal to how a structure's own pieces are generated) — Context §B/§C.
- The structure-starts/structure-references generation flow's shared machinery (biome pre-check, weighted multi-structure selection, idempotent semantics) — Context §D.
- The complete jigsaw assembly engine (Context §F), template loading and the full processor pipeline (Context §H/§I), terrain adaptation and the beardifier density contribution (Context §G), and structure persistence (Context §K).

The 15 hand-coded families are **named, not implemented**: each has its own bespoke piece-generation grammar (a weighted-piece BFS/retry loop for strongholds, a dual weighted-piece grammar for nether fortresses, a recursive grid+corridor layout for woodland mansions, an eager-generation special case for mineshafts, and so on — `06-structures.md` §3.9 describes each at a level of detail this blueprint's own quality bar cannot honestly restate as bit-exact pseudocode without further, dedicated research). This blueprint's `generation::StructureGenerator` trait (Context §D) is the seam **M5-B12** (`M5-B12-structures-tier2`, reserved, not yet drafted — `blueprints/M5/M5-B00-index.md`) implements per family; this blueprint ships exactly one concrete implementor, `jigsaw::JigsawGenerator`, and every hand-coded family's `find_generation_point` call in the meantime returns `StructureGenerationOutcome::Deferred` (Deliverables) rather than panicking or silently producing nothing — a named, honest gap, not a silent one. Structure-set placement, the generation-flow orchestration, persistence, and (for `concentric_rings`-placed families, i.e. strongholds in 26.2) ring-position computation all still work correctly for deferred families; only their own piece geometry is absent until M5-B12 lands.

### B. `random_spread` placement — grid, jitter, frequency reduction, exclusion zones (`docs/research/mc-26.2/06-structures.md` §3.2/§5/§8)

**`potential_structure_chunk`** — the one candidate chunk per `spacing×spacing` grid cell:

```text
fn potential_structure_chunk(world_seed: i64, source_x: i32, source_z: i32, spacing: u32, separation: u32, salt: i64, spread_type: SpreadType) -> (i32, i32):
    grid_x = floor_div(source_x, spacing as i32)   # Rust's i32::div_euclid does NOT match floor_div for negative divisors that never occur here (spacing is always positive), so plain floor division suffices: source_x.div_euclid(spacing as i32) IS floor_div here since the divisor is positive
    grid_z = floor_div(source_z, spacing as i32)
    rng = WorldgenRandom::new(RcLegacyRandom::new(0))   # M5-B01's WorldgenRandom<RcLegacyRandom> — the initial seed 0 is immediately overwritten below, never actually drawn from
    set_large_feature_with_salt(&mut rng, world_seed, grid_x, grid_z, salt)   # M5-B01 §I, called verbatim — this blueprint never re-derives this formula
    limit = (spacing - separation) as i32
    spread_x = spread_type.evaluate(&mut rng, limit)
    spread_z = spread_type.evaluate(&mut rng, limit)
    return (grid_x * spacing as i32 + spread_x, grid_z * spacing as i32 + spread_z)

fn SpreadType::evaluate(&self, rng: &mut impl RcRandomSource, limit: i32) -> i32:
    match self:
        Linear     => rng.next_int_bounded(limit)
        Triangular => (rng.next_int_bounded(limit) + rng.next_int_bounded(limit)) / 2   # i32 division, truncates toward zero
```

`is_random_spread_chunk(world_seed, x, z, placement) = potential_structure_chunk(world_seed, x, z, placement.spacing, placement.separation, placement.salt, placement.spread_type) == (x, z)` — every chunk in a grid cell computes the identical candidate (Context, `06-structures.md` §3.2), so this is a pure, cross-chunk-consistent membership test with no synchronization (GEN-D22).

**Hand-derived known-answer vectors** (this blueprint's own derivation pass, Python, faithful 48-bit-LCG wrapping/masking arithmetic per M5-B01 §C/§D — not independently cross-checked against a live JVM, flagged per this project's own established honesty convention):

| Structure set | `spacing`/`separation`/`salt`/type | `world_seed` | `source_chunk` | grid cell | reseed value | `(spread_x, spread_z)` | candidate chunk |
|---|---|---|---|---|---|---|---|
| Villages | `34/8/10387312`/Linear | `0` | `(0,0)` | `(0,0)` | `10387312` | `(15,2)` | `(15,2)` |
| Villages | `34/8/10387312`/Linear | `0` | `(-5,-5)` | `(-1,-1)` | `-474760728941` | `(25,0)` | `(-9,-34)` |
| Villages | `34/8/10387312`/Linear | `0` | `(100,-37)` | `(2,-2)` | `417960669654` | `(8,8)` | `(76,-60)` |
| Ocean monuments | `32/5/10387313`/Triangular | `12345` | `(0,0)` and `(1,1)` (same cell) | `(0,0)` | `10399658` | `(3,19)` | `(3,19)` |

(All four salts/spacings/separations are the confirmed constants from `06-structures.md` §5's data-generator cross-reference — restated Context §D. Villages' `spread_type` is Linear, the enum's own default, per that same table's silence on the field — `06-structures.md` §7 only names an explicit `spread_type` for the three sets that override it: ocean monuments, woodland mansions, end cities.)

**`FrequencyReductionMethod`** — `applyAdditionalChunkRestrictions`'s extra per-chunk coin-flip, applied only when `placement.frequency < 1.0` (`06-structures.md` §3.2/§8, `HIGHLY_ARBITRARY_RANDOM_SALT = 10387320` per §5). The corpus confirms this constant's role ("legacy_type_2 reducer") and gives `LEGACY_TYPE_1`'s formula exactly but not the other three's precise seeding call — this blueprint's own reconstruction below is **moderate confidence**, flagged for GEN-D27 reconciliation; every acceptance test built against it validates this blueprint's *own* specified algorithm's determinism/behavior, not a vanilla-verified golden vector (the same honesty posture M5-B03's `EndIslands`/M5-B05's `find_spawn_position` already establish):

```text
fn passes_frequency_reduction(world_seed: i64, x: i32, z: i32, method: FrequencyReductionMethod, frequency: f32) -> bool:
    match method:
        LegacyType1 =>
            # confirmed exact formula, §3.2: a bare integer hash, no RandomSource at all
            hash: i32 = wrapping_xor(wrapping_xor(x, wrapping_shl(z, 4)), world_seed as i32)
            # moderate confidence: the corpus does not state the final comparison beyond "a hash" —
            # this blueprint treats it as a direct nextInt-free modulo-style density check, matching
            # the historical "1 in N" pillager-outpost frequency convention: passes iff hash.rem_euclid(100) < (frequency*100.0) as i32
            (hash.rem_euclid(100)) < (frequency * 100.0) as i32
        Default | LegacyType2 | LegacyType3 =>
            rng = WorldgenRandom::new(RcLegacyRandom::new(0))
            set_large_feature_with_salt(&mut rng, world_seed, x, z, HIGHLY_ARBITRARY_RANDOM_SALT)  # = 10387320
            match method:
                Default | LegacyType2 => (rng.next_bits(24) as f64 * (2.0_f64.powi(-24))) < (frequency as f64)   # nextFloat()
                LegacyType3           => rng.next_double() < (frequency as f64)                                   # nextDouble()
```

Hand-derived vector for `LegacyType2` (buried treasure, `frequency = 0.01`, per §5's `spacing=1, separation=0, frequency=0.01 (legacy_type_2)`), `world_seed = 0`: `nextFloat()` at `(0,0)/(1,0)/(2,0)/(3,0)/(4,0)` = `0.719376 / 0.961357 / 0.416874 / 0.751377 / 0.057676` — none pass `< 0.01` at these five coordinates (this blueprint's own derivation; a fixture exercising the passing branch is constructed separately in Acceptance tests using a coordinate/frequency combination this same derivation pass confirmed passes).

**Exclusion zones** (`06-structures.md` §3.2's `applyInteractionsWithOtherStructures`, deprecated but still datapack-declared for pillager outposts vs. villages, §5): a brute-force `(2·range+1)²` chunk-square scan around the candidate, recursively calling `is_random_spread_chunk` against the *other* named structure set's own placement for every cell in that square; if any hit, the candidate is rejected. `range` is the exclusion zone's own `chunk_count` field (`10` for pillager outposts vs. villages, per §5).

### C. `concentric_rings` placement — the stronghold ring algorithm (`06-structures.md` §3.2, confirmed against the ASSET-D18(f) reference — a primary-source restatement, not a third-party-firewall one)

Unlike `random_spread`, ring positions are computed **once per world**, seeded from the legacy level seed (a value distinct from the ordinary per-structure seed — Context §D), not per-candidate-chunk:

```text
fn generate_ring_positions(legacy_level_seed: i64, placement: &ConcentricRingsPlacement, biome_search: &impl RingBiomeSearch) -> Vec<(i32, i32)>:
    rng = RcLegacyRandom::new(legacy_level_seed)
    angle = rng.next_double() * 2.0 * PI               # uniformly random starting angle
    circle = 0_i32                                       # ring index
    positions_in_ring = placement.spread as i32           # starts at the placement's own `spread` field — first ring holds `spread` positions
    remaining_in_current_ring = positions_in_ring
    results = Vec::new()

    for i in 0..placement.count:
        dist = 4.0*(placement.distance as f64)
             + (placement.distance as f64) * (circle as f64) * 6.0
             + (rng.next_double() - 0.5) * (placement.distance as f64) * 2.5
        raw_x = (angle.cos() * dist) as i32 * 32   # chunk-section (32-chunk) units -> chunk units, per §3.2's own "SectionPos.sectionToBlockCoord(_, 8)" note restated at chunk granularity
        raw_z = (angle.sin() * dist) as i32 * 32
        found = biome_search.find_biome_horizontal(
            raw_x * 16, 0, raw_z * 16,   # block coordinates
            112 * 8 * 16,                 # 112 sections * 8 (section=8 chunks) * 16 blocks/chunk — §5's "112 sections" radius restated in blocks
            /* step */ 8 * 16,
            /* matches */ |biome| placement.preferred_biomes contains biome (Context §B's tag-membership seam)
        )
        (final_x, final_z) = found.unwrap_or((raw_x, raw_z))
        results.push((final_x, final_z))

        angle += 2.0 * PI / (positions_in_ring as f64)
        remaining_in_current_ring -= 1
        if remaining_in_current_ring == 0:
            circle += 1
            positions_in_ring += (2 * positions_in_ring) / (circle + 1)
            positions_in_ring = positions_in_ring.min(placement.count as i32 - i as i32 - 1)
            remaining_in_current_ring = positions_in_ring
            angle += rng.next_double() * 2.0 * PI   # extra jitter when starting a new ring
    return results
```

`RingBiomeSearch::find_biome_horizontal` is a resolver seam (Deliverables) a caller implements over M5-B05's `MultiNoiseBiomeSource`/`ClimateSampler` (an expanding-radius scan sampling `biome_at_quart` at the search `step`, returning the first match's block position, or `None` if the whole radius is exhausted) — this blueprint does not implement the search algorithm's own internals, only the seam and the ring-generation loop that calls it, since the search itself is ordinary biome-lookup composition already fully specified by M5-B05.

Strongholds (26.2's only built-in `concentric_rings` consumer, per `06-structures.md` §3.2's own closing line) use `distance=32, spread=3, count=128, salt=0`, biased toward `#minecraft:stronghold_biased_to` (§5). `isPlacementChunk` for a `concentric_rings` set is simply `ring_positions.contains(&(chunk_x, chunk_z))` against the once-computed list.

**Moderate-confidence note, stated once rather than per line**: the exact loop nesting for "grow `positions_in_ring`, cap at remaining count, add jitter" above is this blueprint's own best-effort restatement of `06-structures.md` §3.2's prose description (steps 3–4 of that section's own five-step algorithm), not a literal formula quoted verbatim from that document — the underlying *facts* (ring index growth formula `spread += 2·spread/(circle+1)`, capped at `count − i` remaining, jitter added when moving to a new ring) are confirmed by that section; only their precise loop-structure assembly here is this blueprint's own synthesis, flagged for GEN-D27 reconciliation exactly as that section's own text already anticipates ("confusingly reusing the field name," "this part... is parallelized" — details this blueprint's single-threaded restatement above deliberately does not attempt to reproduce, since GEN-D14's "search implementation is a performance choice" reasoning applies identically here: only the *set* of resulting ring positions is parity-relevant, never how many worker threads computed them).

### D. Structure-set data, generation flow, and biome-check semantics (`06-structures.md` §3.2/§3.3)

From `rc_worldgen::data` (M5-B02, read-only): `StructureSet { placement: StructurePlacement, structures: Vec<StructureSelectionEntry> }`, `StructurePlacement::RandomSpread { salt, spacing, separation, spread_type, frequency_reduction_method, frequency, locate_offset, exclusion_zone }` / `::ConcentricRings { salt, distance, spread, count, preferred_biomes, locate_offset }`, `StructureSelectionEntry { structure: StructureId, weight: u32 }`, `Structure { structure_type: String, biomes: TagOrList<ResourceLocation>, step: DecorationStep, terrain_adaptation: TerrainAdaptation, spawn_overrides: BTreeMap<String, serde_json::Value>, extra: BTreeMap<String, serde_json::Value> }`. `structure.extra` carries every jigsaw-specific field (`start_pool`, `start_jigsaw_name`, `size`, `start_height`, `max_distance_from_center`, `pool_aliases`, `dimension_padding`, `liquid_settings`, `use_expansion_hack`, `project_start_to_heightmap`) as opaque `serde_json::Value` per M5-B02's own deferral (Context, that blueprint) — this blueprint is the first consumer to actually parse those fields, done in `generation::parse_jigsaw_extra` (Deliverables), reading them from the same `serde_json::Value` map with the field names confirmed in `06-structures.md` §3.9/§7 (village plains: `start_pool = "village/plains/town_centers"`, `size=6`, `max_distance_from_center=80`, `project_start_to_heightmap = true`, `use_expansion_hack = true`, `terrain_adaptation = beard_thin`; ancient city: `start_pool = "ancient_city/city_center"`, `start_jigsaw_name = "city_anchor"`, `start_height.absolute = -27`, `size=7`, `max_distance_from_center=116`, `step = underground_decoration`, `terrain_adaptation = beard_box`).

**`STRUCTURE_STARTS`** (`06-structures.md` §3.3), per chunk `(x, z)`: for every `StructureSet` whose member structures' biome lists could possibly intersect the biome source's possible biomes (the caller's own pre-filter — this blueprint accepts an already-filtered `structure_sets` map rather than re-deriving that filter, since it depends on `03`'s biome-source enumeration, out of scope here):

1. Skip the set if any member structure already has a recorded start for this chunk (free under GEN-D26 — this blueprint is a pure function computed once, so "already generated" is a caller-side memoization concern, not code this blueprint needs).
2. If `placement.is_structure_chunk(world_seed, x, z)` is false, skip the set entirely.
3. **Single-structure set**: attempt that one structure directly.
4. **Multi-structure set** (villages/`nether_complexes`/ocean_ruins/ruined_portals/shipwrecks/mineshafts, per §3.3): compute `weighted_draw_order` (below) and try each candidate in turn until one succeeds or the list is exhausted.
5. For each attempted structure: **biome check** — `find_valid_generation_point` first calls the structure kind's own `find_generation_point` (this blueprint's `StructureGenerator::find_generation_point`), then tests the returned stub's position against `structure.biomes` at **quart resolution** (`biome_at(quart_x, quart_y, quart_z)` via M5-B05's `MultiNoiseBiomeSource::biome_at_quart`, checked through the tag-membership seam Context §B already establishes for `TagOrList`) — a candidate whose stub position lands outside the structure's allowed biomes is rejected and, for a multi-structure set, the next weighted candidate is tried instead of aborting the whole set (§3.3: "a failed biome/height check... falls through to try a different weighted pick").

```text
fn weighted_draw_order(world_seed: i64, chunk_x: i32, chunk_z: i32, entries: &[StructureSelectionEntry]) -> Vec<usize>:
    rng = WorldgenRandom::new(RcLegacyRandom::new(0))
    set_large_feature_seed(&mut rng, world_seed, chunk_x, chunk_z)   # seed = world_seed, UNSALTED — M5-B01 §I's own restated call site
    remaining: Vec<usize> = (0..entries.len()).collect()
    order = Vec::new()
    while !remaining.is_empty():
        total_weight: i64 = remaining.iter().map(|&i| entries[i].weight as i64).sum()
        r = rng.next_int_bounded(total_weight as i32) as i64
        running = 0_i64
        for (pos, &idx) in remaining.iter().enumerate():
            running += entries[idx].weight as i64
            if r < running:
                order.push(idx)
                remaining.remove(pos)
                break
    return order
```

**`STRUCTURE_REFERENCES`** (§3.3): for chunk `(x, z)`, scan the full `17×17` neighborhood (`±MAX_STRUCTURE_DISTANCE = 8` in both axes, §5); for every neighbor's own already-generated `StructureStart` whose bounding box intersects `(x,z)`'s 16×16 footprint, record a reference. This is a genuine cross-chunk read dependency (of *coordinates and each neighbor's own generation output*, never of materialized block state — GEN-D22's own distinction) — this blueprint's `scan_structure_references` takes the neighbor-lookup as an injected closure (`&dyn Fn(i32,i32) -> BTreeMap<ResourceLocation, StructureStart>`), matching a future GenStage-integration blueprint's own responsibility to satisfy this as an ordering constraint (prefetch every neighbor to `STRUCTURE_STARTS` before this chunk reaches `STRUCTURE_REFERENCES`, per `docs/research/third-party/rng-parity-notes.md`'s own note on this exact dependency — safe under the "no cross-partition blocking" rule because it is satisfied by ordering/prefetch, never a blocking wait mid-tick, since worldgen is off-tick background work entirely, ARCH-D20/GEN-D25) rather than performing any I/O or region-lookup itself.

`StructureStart::references` (Deliverables) is capped at `MAX_REFERENCES = 1` (`06-structures.md` §5 — vanilla's own base-class default, never overridden by any structure family) — a structure can only ever be "referenced" once in its lifetime; `scan_structure_references`'s own bookkeeping respects this cap.

### E. Structure biome-tag membership — the resolver seam

M5-B02 stores `TagOrList<ResourceLocation>` fields (a structure's `biomes`, a `concentric_rings` placement's `preferred_biomes`) with tag references kept **verbatim, unexpanded** (`#minecraft:foo`), since tag data (`data/minecraft/tags/**`) is outside that blueprint's ten owned JSON families. This blueprint inherits the same deferral rather than inventing a tag-extraction pipeline of its own:

```rust
/// Resolves whether `biome` satisfies a `TagOrList` — a direct list membership check
/// needs no external data; a `Tag(name)` needs an injected tag-membership oracle
/// (Context §E — tag data extraction is a future blueprint's scope, per M5-B02).
pub fn biome_set_contains(
    set: &crate::data::TagOrList<crate::data::ResourceLocation>,
    biome: &crate::data::ResourceLocation,
    tag_membership: &impl Fn(&str, &crate::data::ResourceLocation) -> bool,
) -> bool;
```

### F. The jigsaw system — templates pools, aliases, and the `add_pieces` algorithm (`06-structures.md` §3.5/§3.6)

**Template pools.** `data::TemplatePool { fallback: TemplatePoolId, elements: Vec<(PoolElement, u32)> }` (M5-B02's own compiled shape). `06-structures.md` §3.5: construction **flattens weights into literal list repetition** (a `weight`-copy expansion, not a cumulative-weight scan) — this blueprint's `FlattenedPool::build` performs that expansion once (Deliverables), after which `get_random_template` is a plain `next_int_bounded(flattened.len())` and `get_shuffled_templates` is a **Java `Collections.shuffle`-style** Fisher-Yates shuffle (the standard, publicly-specified JDK algorithm — restated here as a general CS fact, not Mojang-specific expression: for `i` from `len-1` down to `1`, swap `list[i]` with `list[rng.next_int_bounded(i+1)]`) of that same flattened list — reproducing an equivalent-probability-but-differently-shaped selection (e.g. cumulative-weight sampling) desyncs every subsequent draw, per §8's own explicit warning.

Five `PoolElement` kinds (already compiled by M5-B02, unchanged shape): `SinglePoolElement { location, processors, projection }`, `LegacySinglePoolElement` (same, but auto-appends `BlockIgnoreProcessor::STRUCTURE_AND_AIR` instead of `::STRUCTURE_BLOCK` at placement — Context §H), `ListPoolElement { elements, projection }` (places every sub-element at the same anchor; jigsaw connectors come only from `elements[0]`), `FeaturePoolElement { feature, projection }` (out of this blueprint's own scope — splicing an ordinary placed feature into the jigsaw graph needs the feature-placement pipeline, a different M5 blueprint's domain; `JigsawGenerator` treats a resolved `FeaturePoolElement` as contributing zero jigsaw blocks and a 1×1×1 bounding box, a safe, harmless stub — never a panic), `EmptyPoolElement` (terminates a branch; picking it as the **very first** center-pool draw aborts generation entirely, per §3.5).

`Projection::TerrainMatching` implicitly appends a `GravityProcessor { heightmap: "WORLD_SURFACE_WG", offset: -1 }` to the element's own processor chain at placement time (§3.5); `Projection::Rigid` appends nothing.

**Pool aliases** (§3.6) — resolved once per structure-start attempt, before any pool lookup:

```text
fn PoolAliasLookup::create(bindings: &[PoolAliasBinding], structure_start_pos: [i32;3], seed: i64) -> Self:
    factory = RcLegacyRandom::new(seed).fork_positional()   # RandomSource::create(seed) == LegacyRandomSource::new(seed), M5-B01 §B's own factory table — this is the concrete, high-confidence pin for which family backs pool-alias resolution
    random = factory.at(structure_start_pos.x, structure_start_pos.y, structure_start_pos.z)
    map = BTreeMap::new()
    for binding in bindings:   # in JSON list order
        binding.for_each_resolved(&mut random, &mut map)
    return PoolAliasLookup(map)

fn PoolAliasBinding::for_each_resolved(&self, random: &mut impl RcRandomSource, map: &mut BTreeMap<ResourceLocation, ResourceLocation>):
    match self:
        Direct { alias, target } => map.insert(alias.clone(), target.clone())
        Random { alias, targets } =>   # WeightedList<target>, drawn once, same flatten-then-pick discipline as template pools
            chosen = weighted_pick_flattened(targets, random)
            map.insert(alias.clone(), chosen)
        RandomGroup { groups } =>      # WeightedList<List<PoolAliasBinding>>
            chosen_group = weighted_pick_flattened(groups, random)
            for inner in chosen_group: inner.for_each_resolved(random, map)   # SAME random stream — this is what correlates paired aliases (trial chambers' matched spawner mob variants, §3.6)

fn lookup<'a>(&'a self, name: &'a ResourceLocation) -> &'a ResourceLocation:
    self.0.get(name).unwrap_or(name)   # unresolved names pass through unchanged
```

Every pool lookup throughout `add_pieces` below (the center pool and every target pool) is passed through `lookup` first.

**`add_pieces` — the complete algorithm** (§3.5, restated in full):

```text
fn add_pieces(ctx: &JigsawPlacementContext, structure_seed_random: &mut impl RcRandomSource) -> Option<Vec<StructurePiece>>:
    pools = PoolAliasLookup::create(ctx.pool_aliases, ctx.origin, ctx.world_seed)   # or the structure-generation seed the caller derives per GEN-D6 "structure generation itself" (world_seed, unsalted — M5-B01 §I)

    center_pool_id = ctx.template_pool_names[pools.lookup(ctx.start_pool)]
    center_pool = FlattenedPool::build(&ctx.template_pools[center_pool_id])
    center_element = center_pool.get_random_template(structure_seed_random)
    if center_element is EmptyPoolElement: return None   # generation fails immediately, §3.5

    origin = ctx.origin
    if let Some(name) = ctx.start_jigsaw_name:
        jb = center_element.jigsaw_blocks(Rotation::None).into_iter().find(|j| j.name == name)?
        origin = origin - jb.pos   # re-anchor so this jigsaw block lands exactly at ctx.origin

    center_piece = StructurePiece::new_jigsaw(center_element, Rotation::None, origin, gen_depth: 0)
    if ctx.project_start_to_heightmap:
        center_piece.bounding_box.min.y = (ctx.heightmap)(origin.x, origin.z) + center_piece.bounding_box.min.y - origin.y   # project onto WORLD_SURFACE_WG
        # (translate the whole piece by the delta; junction bookkeeping updates identically)
    if !within_dimension_padding(&center_piece.bounding_box, ctx.dimension_padding): return None

    if ctx.max_depth == 0: return Some(vec![center_piece])

    free_space = FreeSpaceTracker::new()   # ambient, shared across the whole placement
    free_space.add(center_piece.bounding_box)
    queue = PriorityBfsQueue::new()        # Context: priority-bucketed FIFO, "SequencedPriorityIterator"
    queue.push(PieceState { piece: center_piece.clone(), depth: 1, own_free_space: FreeSpaceTracker::new() }, priority: 0)
    result = vec![center_piece]

    while let Some(state) = queue.pop():   # highest priority bucket first; FIFO within a bucket
        tries_children(ctx, &pools, state, &mut free_space, &mut queue, &mut result, structure_seed_random)
    return Some(result)

fn try_placing_children(ctx, pools, state, ambient_free_space, queue, result, random):
    jigsaws = state.piece.jigsaw_blocks()   # in the element's own declared order
    shuffled = collections_shuffle(jigsaws, random)
    shuffled.sort_by_key(|j| Reverse(j.selection_priority))   # STABLE sort — ties keep shuffle order, §3.5

    for source_jb in shuffled:
        target_pool_name = pools.lookup(&source_jb.pool)
        target_pool = &ctx.template_pools[ctx.template_pool_names[target_pool_name]]
        fallback_pool = &ctx.template_pools[target_pool.fallback resolved through pools.lookup too]

        candidates: Vec<PoolElement> =
            if state.depth < ctx.max_depth { FlattenedPool::build(target_pool).all() } else { Vec::new() }
        candidates_shuffled = collections_shuffle(candidates, random)
        fallback_shuffled = collections_shuffle(FlattenedPool::build(fallback_pool).all(), random)

        placed = false
        for candidate in candidates_shuffled.iter().chain(fallback_shuffled.iter()):
            rotations_shuffled = collections_shuffle(Rotation::all(), random)   # FRESH shuffle per candidate — Context, flagged moderate-confidence on cadence
            for rotation in rotations_shuffled:
                for target_jb in candidate.jigsaw_blocks(rotation):   # candidate's own declared order (not shuffled — only the SOURCE side shuffles jigsaw blocks, §3.5's own text names only the source piece's list as shuffled)
                    if !can_attach(&source_jb, &target_jb): continue
                    (new_pos, uses_own_free_space) = compute_vertical_placement(&state.piece, &source_jb, &candidate, &target_jb, rotation, ctx.heightmap)
                    candidate_box = candidate.bounding_box(rotation, new_pos)
                    if ctx.use_expansion_hack: candidate_box = expand_for_headroom(candidate_box, candidate, target_pool)
                    free_space_ref = if uses_own_free_space { &mut state.own_free_space } else { ambient_free_space }
                    if free_space_ref.collides(candidate_box, deflate: 0.25): continue
                    if !within_dimension_padding(&candidate_box, ctx.dimension_padding): continue

                    new_piece = StructurePiece::new_jigsaw(candidate.clone(), rotation, new_pos, gen_depth: state.depth)
                    junction = JigsawJunction { source_x: source_jb.world_pos.x, source_ground_y: (ctx.heightmap)(source_jb.world_pos.x, source_jb.world_pos.z), source_z: source_jb.world_pos.z, delta_y: new_pos.y - source_jb.world_pos.y, projection: candidate.projection };
                    state.piece.junctions.push(junction.clone()); new_piece.junctions.push(junction.reversed());
                    free_space_ref.add(candidate_box)
                    result.push(new_piece.clone())
                    if state.depth < ctx.max_depth:
                        queue.push(PieceState { piece: new_piece, depth: state.depth + 1, own_free_space: FreeSpaceTracker::new() }, priority: source_jb.placement_priority)
                    placed = true
                    break  # `continue label129` — abort remaining rotations/candidates for THIS source jigsaw
                if placed: break
            if placed: break
        # no `else` needed — a source jigsaw with no valid attachment simply contributes no child; the branch dead-ends there
```

**`can_attach`** — the asymmetric rule, restated exactly (§3.5, §8's own explicit warning against a naive bidirectional check): `source.front == target.front.opposite() AND (source.joint == Rollable OR source.top == target.top) AND source.target == target.name` — **only** the source's own `target` field is checked against the candidate's `name`; the candidate's own `target` field is never consulted.

**Vertical placement**: rigid-source-to-rigid-target connections stack by fixed jigsaw-block Y offsets relative to the source piece's own box (`uses_own_free_space = true` whenever the target jigsaw's connector position falls inside the source piece's own bounding box, per §3.5's precise rule — "if the target jigsaw's connector lands *inside* the source piece's own box, the source's private free-space shape is reused"); any placement involving a `TerrainMatching`-projected piece instead re-samples `ctx.heightmap` at the source jigsaw's own column and anchors relative to that surface height.

**Expansion hack** (`use_expansion_hack`, villages only, §3.5): before the collision test, any candidate whose own box is `≤16` blocks tall has its box inflated (one recursion level via each pool's own cached `get_max_size` — the tallest non-empty element's Y-span) to reserve headroom a taller follow-on piece might need.

**Depth/range constants**: `MIN_DEPTH = 0`, `MAX_DEPTH_CAP = 20` (the datapack `size` field's own codec range), `MAX_TOTAL_STRUCTURE_RANGE = 128`, `terrain_adaptation_edge_padding = 12` for any non-`None` `TerrainAdaptation` (`0` otherwise) — `max_distance_from_center + terrain_adaptation_edge_padding` must not exceed `128` (a structure JSON violating this is a data-pipeline validation concern, not this blueprint's runtime check, though `add_pieces` itself never needs to enforce it since `dimension_padding`/the caller's own `max_depth` already bound recursion depth).

### G. Terrain adaptation and the beardifier density contribution

`TerrainAdaptation` (M5-B02's compiled enum, unchanged): `None`, `Bury`, `BeardThin`, `BeardBox`, `Encapsulate`. Any non-`None` value inflates the structure's **reported** bounding box by `12` blocks in every direction for range-check purposes (Context §F) — this is a *reporting* inflation, not a change to the piece's own placed geometry.

**Beardifier** (`docs/research/mc-26.2/05-worldgen.md` §4's constants table: kernel radius `12` blocks, a `24³` = `13824`-entry precomputed kernel; §3.4: "converts nearby rigid structure pieces + jigsaw junctions into a density perturbation so terrain doesn't clip through them," 4 `TerrainAdjustment` behaviors). **This formula is moderate confidence** — the research corpus confirms the kernel's radius/size and that it is a genuine density contribution summed into `final_density` (not a separate carve/fill pass), but does not give the literal per-cell kernel values or exact per-piece contribution shape; this blueprint's own reconstruction, flagged for GEN-D27 reconciliation:

```rust
pub const BEARD_KERNEL_RADIUS: i32 = 12;

/// Moderate-confidence reconstruction (Context §G) of vanilla's per-position beardifier
/// contribution, added directly into `final_density` at the point `Beardifier{}` appears
/// in the noise router graph (M5-B03 GEN-D12). For each nearby piece/junction within
/// `BEARD_KERNEL_RADIUS` blocks, contributes a falloff term that pushes density toward
/// "solid" (a `Bury`/`BeardBox`-adapted piece's interior) or "air" (clearing space around
/// a `BeardThin`-adapted piece's edges), scaled by inverse-distance-squared within the
/// kernel radius and zero beyond it. `Encapsulate` and `None` contribute nothing (a
/// non-adapted piece relies purely on its own block-stamping pass, never a density
/// perturbation).
pub fn sample_beardifier(ctx: &BeardifierContext, block_x: i32, block_y: i32, block_z: i32) -> f64;
```

**Wiring into M5-B03** (modifications to already-delivered files, not a new module of M5-B03's own): `crates/worldgen/src/density/interpreter.rs`'s `evaluate_node` free function gains one trailing parameter, `beardifier: Option<&beardifier::BeardifierContext<'_>>`; its `Beardifier{}` arm becomes `beardifier.map(|b| beardifier::sample_beardifier(b, ctx.block_x, ctx.block_y, ctx.block_z)).unwrap_or(0.0)` (preserving the exact prior `0.0` behavior whenever `None` is passed — every existing Tier-1 call site that has no beardifier context in scope, e.g. a bare spawn-point search, continues to behave identically). `DensityInterpreter<'a>::sample` gains a `beardifier: Option<&'a beardifier::BeardifierContext<'a>>` field (constructor parameter, `Deliverables`) it forwards to `evaluate_node`. `crates/worldgen/src/density/noise_chunk.rs`'s `NoiseChunk<'a>` gains the identical field, set once at `NoiseChunk::new` (a per-chunk construction, matching Beardifier's own per-chunk nature — unlike `NoiseGraphState`, which is per-world and therefore cannot hold this), forwarded at every one of its own internal `evaluate_node` call sites (the corner-fill loop, `fill_all_directly`). Both files' Constraints (a) — "no Mojang or third-party reimplementation code" — are unaffected: this modification only threads a new parameter through an already-specified dispatch function per this blueprint's own, separately-derived formula.

### H. Structure NBT templates — format and runtime loading (`06-structures.md` §3.7, GEN-D23)

**Format** (root compound): `size` (3 ints), `palette`/`palettes` (list of block-state palette entries; `palettes` — plural — when the template ships multiple randomly-selectable palette variants, one resolved per `StructurePlaceSettings` via `get_random_palette`), `blocks` (list of `{pos: [x,y,z], state: paletteIndex, nbt?: block-entity compound}`), `entities` (list of `{pos: [3 doubles], blockPos: [3 ints], nbt: entity compound}`), `DataVersion` (int, informational only at this milestone — no DFU exists yet, matching M2-B04's own DataVersion policy: refuse anything that isn't the pinned `4903`, never silently migrate).

**Runtime loading** (GEN-D23, restated exactly): Rusty Clanker never ships template bytes. The operator supplies a legally-obtained vanilla `server.jar` or its extracted `data/minecraft/structure/**.nbt` tree at server startup; this blueprint's `DirectoryTemplateSource` reads gzip-compressed `.nbt` files directly from that operator-supplied root at chunk-generation time, via `rc_nbt::read_gzip_owned` (M2-B02's own already-committed function — no new dependency beyond the one `rc-nbt` path-dependency edge this blueprint adds to `rc-worldgen`'s `Cargo.toml`, Deliverables). Layout: `<root>/<namespace>/<path>.nbt` — the jar's own internal `data/<namespace>/structure/<path>.nbt` tree with the `structure/` path segment dropped (this closes `08-assets-auth-legal.md`'s own flagged open interface item, "the canonical on-disk path/format `04` expects `data/vanilla-structures/` to expose" — `data/vanilla-structures/` is the recommended root value, though this blueprint's own `DirectoryTemplateSource::root` field takes any operator-configured path, not a hardcoded one). `CachingTemplateSource<S>` wraps any `TemplateSource` with an unbounded, invalidated-only-on-restart cache (`BTreeMap<ResourceLocation, Option<StructureTemplate>>`), mirroring `06-structures.md` §3.7's own `StructureTemplateManager` caching behavior.

**Placement** (`StructureTemplate.placeInWorld`, §3.7, the single choke point every pool element routes through): resolve palette → run every block through the processor pipeline (Context §I) → place blocks (setting a temporary marker state first for any block carrying block-entity NBT, avoiding a transient invalid state — this blueprint's `place_in_world` accepts a caller-supplied `barrier_state: BlockStateId` for this purpose rather than hardcoding vanilla's own barrier block) → a flood-fill liquid-reconciliation pass for any fluid displaced by the template (a fixed-point loop over queued positions pulling from adjacent source-fluid neighbors until stable) → shape recalculation at the placed region's edge (skipped when `settings.known_shape` is true — always true for jigsaw/pool placement, since bounding boxes are already known-safe) → entity placement (position/yaw transformed by rotation+mirror, a fresh entity id assigned, never the template's own saved one).

### I. Structure processors — complete enumeration (`06-structures.md` §3.8)

`StructureProcessor::process_block` may replace, keep, or cancel (`None` return) a block as it is stamped; a processor additionally implementing `finalize_processing` (a whole-list post-pass) and reporting `evaluates_entire_piece_state() == true` switches `processBlockInfos` from a "clip to the current chunk's bounding box during iteration" fast path to a "process every block across the whole piece regardless of chunk bounds" slow path — only `Capped` needs this, since it must see every candidate before picking `N` to replace.

| Processor | Effect |
|---|---|
| `BlackstoneReplace` | Unconditional 1:1 substitution table (cobblestone→blackstone, stone→polished blackstone, stone bricks→polished blackstone bricks + their stairs/slabs/walls, chiseled/cracked variants, iron bars→iron chain — ~20 mappings), preserving stair/slab shape properties. |
| `BlockAge { mossiness }` | Per-block-type coin-flip: 50% chance to swap full stone-brick blocks for cracked/stairs variants (further mossy-vs-non-mossy split by `mossiness`), 50% chance to age stairs, `mossiness`-chance to moss slabs/walls, 15% chance obsidian→crying obsidian. |
| `BlockIgnore { blocks }` | Drops any matching block. |
| `BlockRot { rottable_blocks, integrity }` | Per-block coin-flip (seeded from `Mth::get_seed(world_pos)`, gated to `rottable_blocks` if set) to drop the block; `integrity` is the *keep* probability. |
| `Capped { delegate, limit }` | Runs `delegate` against only a random `limit`-sized subset of the whole piece's blocks (positional-random subset selection). `evaluates_entire_piece_state() = true`. |
| `Gravity { heightmap, offset }` | Re-projects onto the live-world heightmap at its column plus `offset`, preserving relative template height. |
| `JigsawReplacement` | Replaces every placed `JIGSAW` block with the state parsed from its `final_state` tag (or air), unless `keep_jigsaws`; `final_state = structure_void` drops the block entirely. Auto-appended by `SinglePoolElement`/`LegacySinglePoolElement` unless `keep_jigsaws`. |
| `LavaSubmergedBlock` | If the target world position is already lava and the incoming block isn't a full cube, forces the placed block to stay lava. |
| `Nop` | Identity. |
| `ProtectedBlocks { value }` | Cancels placement if the **existing world block** matches the protected set. |
| `Rule { rules }` | The general conditional-replace processor (below). |

`ProcessorRule = (input_predicate: RuleTest, location_predicate: RuleTest, position_predicate: Option<PosRuleTest>, output_state, block_entity_modifier)`. Fires when the template's own block passes `input_predicate`, the current world block at the target position passes `location_predicate`, and `position_predicate` (default `AlwaysTrue`) passes given `(template_relative_pos, world_pos, reference_pos)`; on success the block becomes `output_state`. `RuleProcessor` seeds its per-block RNG from `Mth::get_seed(world_pos)` — M5-B01's own `mth_get_seed` function, called verbatim, never re-derived — deterministic per world position, independent of processing order.

`RuleTest` (6 kinds): `AlwaysTrue`, `BlockMatch { block }` (exact block match, ignores state properties), `BlockstateMatch { block_state }` (exact state match), `TagMatch { tag }`, `RandomBlockMatch { block, probability }` / `RandomBlockstateMatch { block_state, probability }` (as their non-random counterparts, ANDed with `rng.next_float() < probability`).

`PosRuleTest` (3 kinds): `AlwaysTrue`, `LinearPos { min_chance, max_chance, min_dist, max_dist }` (probability linearly interpolated over Manhattan distance from `reference_pos`), `AxisAlignedLinearPos { .., axis }` (same interpolation, measured along one axis only).

```text
fn run_processor_list(processors: &[StructureProcessor], ctx: &ProcessorContext, blocks: Vec<PlacedBlockInfo>, chunk_bounds: Option<BoundingBox>) -> Vec<PlacedBlockInfo>:
    working = if any(p.evaluates_entire_piece_state()) { blocks } else { blocks.into_iter().filter(|b| chunk_bounds.map_or(true, |cb| cb.contains(b.pos))).collect() }
    for processor in processors:   # in declared order — datapack list order, then the two auto-appended processors last (BlockIgnore, then JigsawReplacement, §3.5)
        working = working.into_iter().filter_map(|b| processor.process_block(ctx, b)).collect()
        if processor.evaluates_entire_piece_state(): processor.finalize_processing(ctx, &mut working)
    return working
```

### J. Non-jigsaw structure families — named, deferred

| `StructureType` id | One-line description (`06-structures.md` §3.9) | Placement kind |
|---|---|---|
| `stronghold` | `concentric_rings` placement (ring math **fully implemented**, Context §C) + a retry-until-portal-room weighted piece BFS (`MAX_DEPTH=50`, `STRONGHOLD_PIECE_WEIGHTS`, salt-incrementing retry loop) | `concentric_rings` |
| `fortress` (nether fortress) | Dual weighted-piece grammar (bridge vs. castle tables), `MAX_DEPTH=30`, fixed Y-band placement | `random_spread` (`nether_complexes` set, shared with `bastion_remnant`) |
| `woodland_mansion` | Fixed 11×11 room grid, recursive corridor carving, three per-floor `FloorRoomCollection`s | `random_spread` |
| `ocean_monument` | Single procedural room-grid piece, `29×29`-block full-surrounding-biome gate | `random_spread` |
| `mineshaft` | Eager piece-tree generation (the one structure that builds its full tree inside `find_generation_point` rather than lazily), two skins (normal/mesa) | `random_spread`, degenerate (`spacing=1, separation=0`, `frequency=0.004`) |
| `end_city` | Recursive tower with chance-based side/ship/gateway extensions, template-driven, no jigsaw | `random_spread` |
| `desert_pyramid` / `jungle_temple` / `swamp_hut` | Single procedural `ScatteredFeaturePiece` each | `random_spread` |
| `igloo` | Template-based, occasional igloo+basement two-piece variant | `random_spread` |
| `ocean_ruin` | Warm/cold biome-temperature template variants, cluster probability | `random_spread` |
| `shipwreck` | Template-based, terrain-oriented | `random_spread` |
| `buried_treasure` | Single fixed procedural piece at chunk center on ocean floor | `random_spread`, degenerate |
| `ruined_portal` | Up to 7 biome-flavored template `Setup` variants, vertical-placement modes | `random_spread` |
| `nether_fossil` | Vertical stack of template bone segments | `random_spread`, near-continuous grid |

Every family above is a valid `Structure::structure_type` string this blueprint's `StructureStartContext` may encounter; `generation::dispatch_generator` (Deliverables) recognizes each by name and returns `StructureGenerationOutcome::Deferred(structure_type)` rather than an error — structure-set placement, biome pre-check, and persistence machinery all still function correctly around a deferred family; only its own piece geometry is absent.

### K. Persistence — the `structures` chunk-NBT compound (`docs/research/mc-26.2/04-persistence-nbt.md` §3.6/§3.7, cross-checked against `minecraft.wiki`'s public Chunk format page, fetched during this blueprint's own derivation pass)

Root shape (confirmed, both sources agree): `structures: { starts: Compound, References: Compound }`. `References`'s entries are `<structureId>: LongArray`, each `i64` packing a referencing chunk's `(x, z)` — `x` in the low 32 bits, `z` in the high 32 bits (confirmed, both sources): `pack_chunk_pos(x, z) = (z as i64) << 32 | (x as u32 as i64)`; `MAX_STRUCTURE_DISTANCE = 8` bounds which entries are valid (`04-persistence-nbt.md`'s own constants table: "Max Chebyshev chunk distance for a valid structure-reference entry").

`starts`'s entries are `<structureId>: StructureStart-compound`. Confirmed fields (public wiki documentation): `id: String` (the structure's own namespaced id, or the literal string `"INVALID"` when this chunk has no valid start for that structure — `ChunkX`/`ChunkZ`/`Children` are then absent entirely, not merely empty), `ChunkX: Int`, `ChunkZ: Int`, `Children: List<Compound>` (the piece list), `Processed: List` (monument-specific, out of this blueprint's own scope — always written as an empty list, `06-structures.md` §J's deferred-family policy applied here too). Confirmed per-piece fields: `BB: IntArray[6]` (`minX,minY,minZ,maxX,maxY,maxZ`), `id: String` (the piece-type identifier — always the literal `"jigsaw"` for every piece this blueprint's own `JigsawGenerator` produces), `O: Int` (orientation — **for a `jigsaw`-typed piece specifically, this blueprint stores the piece's `Rotation` ordinal directly, `0..3`**, since `PoolElementStructurePiece` carries its own `Rotation` field rather than the six-way `Orientation` enum non-jigsaw pieces use — a moderate-confidence application of the generic `O` field to jigsaw's own different rotation representation, flagged for reconciliation), `GD: Int` (gen depth).

**Jigsaw-specific piece fields** — the wiki's own confirmed example (`PosX`/`PosY`/`PosZ` for *village parts*, a non-jigsaw legacy family) demonstrates that per-family extra fields are expected but does not give jigsaw's own set; this blueprint's own moderate-confidence design, under clearly own-namespaced keys so a future reconciliation pass can rename without ambiguity: `pool_element: Compound` (`kind: String` one of `single`/`legacy_single`/`list`/`feature`/`empty`; `location: String` for single/legacy-single; `processors: String` optional; `projection: String`), `ground_level_delta: Int`, `junctions: List<Compound>` (`x`/`ground_y`/`z`/`delta_y`: Int each).

```rust
pub const MAX_REFERENCES: u32 = 1;

pub fn pack_chunk_pos(x: i32, z: i32) -> i64;
pub fn unpack_chunk_pos(packed: i64) -> (i32, i32);

pub fn encode_structures_compound(
    starts: &std::collections::BTreeMap<crate::data::ResourceLocation, crate::structure::generation::StructureStart>,
    references: &std::collections::BTreeMap<crate::data::ResourceLocation, Vec<(i32, i32)>>,
) -> rc_nbt::owned::NbtCompound;

#[derive(Debug, thiserror::Error)]
pub enum StructurePersistenceError {
    #[error("malformed structures compound: {0}")]
    Malformed(String),
}

pub fn decode_structures_compound(
    compound: &rc_nbt::owned::NbtCompound,
) -> Result<
    (
        std::collections::BTreeMap<crate::data::ResourceLocation, crate::structure::generation::StructureStart>,
        std::collections::BTreeMap<crate::data::ResourceLocation, Vec<(i32, i32)>>,
    ),
    StructurePersistenceError,
>;
```

**Integration with `rc-chunk-storage`'s `ChunkNbtDocument` (M2-B04's own file, `crates/chunk-storage/src/chunk_nbt.rs`)**: `rc-chunk-storage` cannot depend on `rc-worldgen` (the dependency edge runs the other way, `12-workspace-structure.md`'s crate graph, unchanged by this blueprint), so `ChunkNbtDocument` cannot hold a typed `StructureStart`. Instead, `ChunkNbtDocument.structures` (previously absent — M2-B04 always wrote a hardcoded `{starts: {}, References: {}}` literal and discarded any loaded value, its own named "fixed-default field" gap) becomes a **raw, opaque** `rc_nbt::owned::NbtCompound` field: `to_nbt` writes it verbatim under the `structures` key (a caller with no real structure data yet passes an empty compound, exactly reproducing M2-B04's own prior fixed-default behavior — full backward compatibility, no round-trip test elsewhere in that blueprint's own suite is broken); `from_nbt` captures the loaded `structures` compound into this field instead of discarding it. `rc-worldgen`'s `structure::persistence::encode_structures_compound`/`decode_structures_compound` are the actual typed bridge a future GenStage-integration caller uses on either side of that opaque field. This is a minimal, surgical change to M2-B04's file — one field's type and two lines of pass-through logic, no new dependency, no change to `Cargo.toml`.

### L. Region/cluster border locality (GEN-D22, restated and confirmed to hold)

Every algorithm in Context §B–§G is a pure function of `(world seed, coordinates, bounded static parameters read from the compiled `WorldgenData`)`: placement math never reads block state; the generation flow's biome check samples the noise router (M5-B05/M5-B03, themselves pure); jigsaw assembly's only external input is `ctx.heightmap` (a pure terrain-height query, never a block read); template placement and processors read/write only the piece's own bounding box's blocks. When a structure's piece geometry spans a region or cluster-node boundary, each owning partition independently recomputes the identical `add_pieces` output (cheap: pure seed/coordinate math) and stamps only its own sub-volume — exactly GEN-D22's own claim, and this blueprint's algorithms are the concrete mechanism that makes it true rather than merely asserted.

### M. Porting-pitfall checklist (condensed, all already resolved above)

1. **`WorldgenRandom::new(RcLegacyRandom::new(0))`'s initial `0` seed is always immediately overwritten** by the very next `set_large_feature_with_salt`/`set_large_feature_seed` call in every algorithm above — never draw from it before that call.
2. **`FrequencyReductionMethod` variants are not vestigial** — all four are live in 26.2's own built-in structure sets (Context §B).
3. **Weighted pool/alias selection is draw-count-sensitive** — always flatten-then-`next_int_bounded`, never a cumulative-weight scan (Context §F).
4. **`can_attach` is asymmetric** — only the source's `target` field is checked; never add a reverse check (Context §F).
5. **Depth cutoff still consults the fallback pool** — a hard stop at `max_depth` without trying fallback produces dangling connectors vanilla never has (Context §F).
6. **`SequencedPriorityIterator` is priority-bucketed FIFO**, neither plain BFS nor a plain heap — fully drain one priority bucket before the next (Context §F).
7. **Two independent seeds feed one world's structures**: the per-chunk placement seed (`world_seed` directly) and the concentric-rings seed (`legacy_level_seed`, kept distinct for stronghold save-format compatibility, Context §C) — never conflate them.
8. **`STRUCTURE_REFERENCES`'s 17×17 scan is a legitimate cross-chunk read of *coordinates and neighbor generation output*, never of materialized block state** — safe under the "no cross-partition blocking" rule only when satisfied by ordering/prefetch (Context §D).
9. **`Mth::get_seed` (M5-B01's own function) is the exact per-block seed `RuleProcessor` uses** — never a fresh, ad hoc hash.
10. **`rc-chunk-storage` never depends on `rc-worldgen`** — the `structures` NBT field stays opaque at that layer; only `rc-worldgen` interprets it (Context §K).

## Deliverables

### `crates/worldgen/Cargo.toml` (modify — add one internal dependency)

```toml
[dependencies]
rc-core = { path = "../core" }
rc-chunk-storage = { path = "../chunk-storage" }
rc-registries = { path = "../registries" }
rc-nbt = { path = "../nbt" }               # M5-B08: structure template parsing + persistence codec
rand_xoshiro = { workspace = true }
md-5 = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
postcard = { workspace = true }
```

(Every line except `rc-nbt` is M5-B01/M5-B02's existing content, reproduced for a complete file.)

### `crates/worldgen/src/lib.rs` (modify — one new top-level module)

```rust
pub mod structure;
```

### `crates/worldgen/src/structure/mod.rs` (new)

```rust
//! Structure generation (GEN-D21–D23): structure-set placement, the structure-starts/
//! structure-references generation flow, jigsaw assembly, template/processor handling,
//! the beardifier density contribution, and chunk-NBT persistence. See this module's
//! owning blueprint (`M5-B08`) for the full derivation. Non-jigsaw (hand-coded) structure
//! families are named but not implemented here (Context §A/§J).

pub mod beardifier;
pub mod generation;
pub mod jigsaw;
pub mod persistence;
pub mod placement;
pub mod processor;
pub mod template;

pub use beardifier::{BeardifierContext, JunctionBeardSource, PieceBeardSource, BEARD_KERNEL_RADIUS};
pub use generation::{
    BoundingBox, StructureBlockSink, StructureGenerationOutcome, StructureGenerator,
    StructureStart, StructureStartContext, StructurePiece,
};
pub use jigsaw::{
    add_pieces, JigsawGenerator, JigsawPlacementContext, Mirror, PoolAliasBinding,
    PoolAliasLookup, Rotation,
};
pub use persistence::{
    decode_structures_compound, encode_structures_compound, pack_chunk_pos,
    unpack_chunk_pos, StructurePersistenceError, MAX_REFERENCES,
};
pub use placement::{
    biome_set_contains, passes_frequency_reduction, potential_structure_chunk,
    weighted_draw_order, ConcentricRingsPlacement, FrequencyReductionMethod,
    RandomSpreadPlacement, RingBiomeSearch, SpreadType,
};
pub use processor::{
    apply_processor, eval_pos_rule_test, eval_rule_test, run_processor_list,
    PlacedBlockInfo, ProcessorContext,
};
pub use template::{
    CachingTemplateSource, DirectoryTemplateSource, StructurePlaceSettings, StructureTemplate,
    TemplateBlockInfo, TemplateEntityInfo, TemplateParseError, TemplateSource,
};
```

### `crates/worldgen/src/structure/placement.rs` (new)

```rust
use crate::data::ResourceLocation;
use crate::random::{RcLegacyRandom, RcRandomSource, WorldgenRandom};

pub const HIGHLY_ARBITRARY_RANDOM_SALT: i32 = 10387320;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SpreadType { #[default] Linear, Triangular }
impl SpreadType {
    /// Context §B. Draws exactly one (`Linear`) or two (`Triangular`) `next_int_bounded`
    /// calls, X-then-Z order enforced by the caller (`potential_structure_chunk`).
    pub fn evaluate(&self, rng: &mut impl RcRandomSource, limit: i32) -> i32;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrequencyReductionMethod { Default, LegacyType1, LegacyType2, LegacyType3 }

pub struct RandomSpreadPlacement {
    pub salt: i64,
    pub spacing: u32,
    pub separation: u32,
    pub spread_type: SpreadType,
    pub frequency_reduction_method: Option<FrequencyReductionMethod>,
    pub frequency: Option<f32>,
    pub locate_offset: [i32; 3],
}

/// Context §B. The one candidate chunk per grid cell.
pub fn potential_structure_chunk(
    world_seed: i64, source_x: i32, source_z: i32,
    spacing: u32, separation: u32, salt: i64, spread_type: SpreadType,
) -> (i32, i32);

/// `potential_structure_chunk(..) == (x, z)` — Context §B.
pub fn is_random_spread_chunk(world_seed: i64, x: i32, z: i32, p: &RandomSpreadPlacement) -> bool;

/// Context §B. Moderate confidence for every variant except `LegacyType1` (see Context).
pub fn passes_frequency_reduction(
    world_seed: i64, x: i32, z: i32, method: FrequencyReductionMethod, frequency: f32,
) -> bool;

/// Context §B's brute-force `(2*range+1)^2` scan. `other_set_check` re-invokes
/// `is_random_spread_chunk` (or a caller-supplied equivalent for a `concentric_rings`
/// other-set, not needed by any vanilla exclusion zone in 26.2 per Context §B) against
/// the excluded set's own placement.
pub fn violates_exclusion_zone(
    x: i32, z: i32, range: i32, other_set_check: impl Fn(i32, i32) -> bool,
) -> bool;

pub struct ConcentricRingsPlacement {
    pub salt: i64,
    pub distance: u32,
    pub spread: u32,
    pub count: u32,
    pub preferred_biomes: crate::data::TagOrList<ResourceLocation>,
    pub locate_offset: [i32; 3],
}

/// Context §C. `biome_search` is the caller-supplied resolver seam over M5-B05.
pub trait RingBiomeSearch {
    fn find_biome_horizontal(
        &self, block_x: i32, block_y: i32, block_z: i32, radius_blocks: i32, step: i32,
        matches: &dyn Fn(&ResourceLocation) -> bool,
    ) -> Option<(i32, i32)>;
}

/// Context §C. Computed once per world; the caller (a future world-load blueprint) owns
/// caching this result per `ConcentricRingsPlacement` instance.
pub fn generate_ring_positions(
    legacy_level_seed: i64, placement: &ConcentricRingsPlacement,
    biome_search: &impl RingBiomeSearch,
) -> Vec<(i32, i32)>;

/// Context §D. `WorldgenRandom<RcLegacyRandom>`-seeded weighted draw-without-replacement
/// order for a multi-structure `StructureSet`.
pub fn weighted_draw_order(
    world_seed: i64, chunk_x: i32, chunk_z: i32,
    entries: &[crate::data::StructureSelectionEntry],
) -> Vec<usize>;

/// Context §E.
pub fn biome_set_contains(
    set: &crate::data::TagOrList<ResourceLocation>, biome: &ResourceLocation,
    tag_membership: &impl Fn(&str, &ResourceLocation) -> bool,
) -> bool;
```

### `crates/worldgen/src/structure/generation.rs` (new)

```rust
use crate::data::{DecorationStep, ResourceLocation, Structure, StructureId, TerrainAdaptation};
use std::collections::BTreeMap;

pub const MAX_STRUCTURE_DISTANCE: i32 = 8;

/// Axis-aligned, block-integer bounding box (Context §D — mirrors vanilla's own
/// `BoundingBox` shape, `06-structures.md` §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundingBox { pub min: [i32; 3], pub max: [i32; 3] }
impl BoundingBox {
    pub fn from_corners(a: [i32; 3], b: [i32; 3]) -> Self;
    pub fn intersects(&self, other: &BoundingBox) -> bool;
    /// `deflate_by` shrinks `self` on every face before testing intersection (Context §F's
    /// `0.25`-deflated collision test — expressed here in whole blocks; the caller passes
    /// `0` for an exact test and treats the 0.25 fractional deflation as "touching but not
    /// overlapping is allowed," implemented as `intersects` using `<` rather than `<=` at
    /// every face comparison rather than a literal fractional shrink).
    pub fn intersects_deflated(&self, other: &BoundingBox) -> bool;
    pub fn inflated_by(&self, n: i32) -> BoundingBox;
    pub fn encapsulate(&mut self, other: &BoundingBox);
    pub fn contains(&self, pos: [i32; 3]) -> bool;
    pub fn intersects_chunk(&self, chunk_x: i32, chunk_z: i32) -> bool;
}

/// A generated piece — this blueprint ships only `PieceKind::Jigsaw`; a future hand-coded
/// blueprint adds sibling variants (Context §A/§J).
#[derive(Clone, Debug)]
pub struct StructurePiece {
    pub bounding_box: BoundingBox,
    pub gen_depth: u32,
    pub kind: PieceKind,
}
#[derive(Clone, Debug)]
pub enum PieceKind { Jigsaw(crate::structure::jigsaw::JigsawPieceData) }

#[derive(Clone, Debug)]
pub struct StructureStart {
    pub structure: StructureId,
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub pieces: Vec<StructurePiece>,
    /// Capped at `persistence::MAX_REFERENCES` (Context §D).
    pub references: u32,
}
impl StructureStart {
    pub fn is_valid(&self) -> bool { !self.pieces.is_empty() }
}

/// The block-write seam a `StructureGenerator`'s template-stamping step writes through —
/// injected by the caller (a future GenStage-integration blueprint owning real chunk
/// access), never a direct `rc_chunk_storage::BlockStateColumn` reference (Context §D/§L —
/// a piece's own footprint may span multiple, possibly not-yet-owned chunks).
pub trait StructureBlockSink {
    fn set_block(&mut self, world_x: i32, world_y: i32, world_z: i32, state: crate::data::BlockStateSpec);
    fn get_block(&self, world_x: i32, world_y: i32, world_z: i32) -> Option<crate::data::BlockStateSpec>;
}

pub enum StructureGenerationOutcome {
    Generated(StructureStart),
    /// The biome/height check failed at every attempted candidate (multi-structure sets)
    /// or the single candidate (single-structure sets) — Context §D.
    NoValidPoint,
    /// Named, not implemented (Context §A/§J) — carries the `structure_type` string for
    /// diagnostics.
    Deferred(String),
}

/// Context §A. Implemented once, by `jigsaw::JigsawGenerator`.
pub trait StructureGenerator {
    fn find_generation_point(
        &self, structure: &Structure, world_seed: i64, chunk_x: i32, chunk_z: i32,
        biome_at: &dyn Fn(i32, i32, i32) -> ResourceLocation,
        tag_membership: &dyn Fn(&str, &ResourceLocation) -> bool,
    ) -> StructureGenerationOutcome;
}

/// Dispatches by `structure.structure_type` — Context §J's table.
pub fn dispatch_generator<'a>(
    structure_type: &str, jigsaw: &'a dyn StructureGenerator,
) -> &'a dyn StructureGenerator; // returns `jigsaw` for `"minecraft:jigsaw"`; every other
                                  // id resolves to a small internal `DeferredGenerator` that
                                  // always returns `StructureGenerationOutcome::Deferred`

pub struct StructureStartContext<'a> {
    pub world_seed: i64,
    pub structure_sets: &'a BTreeMap<ResourceLocation, crate::data::StructureSet>,
    pub structures: &'a BTreeMap<StructureId, Structure>,
    pub structure_names: &'a BTreeMap<String, StructureId>,
}

/// Context §D's full `STRUCTURE_STARTS` flow for one chunk.
pub fn generate_structure_starts(
    ctx: &StructureStartContext,
    chunk_x: i32, chunk_z: i32,
    biome_at: &dyn Fn(i32, i32, i32) -> ResourceLocation,
    tag_membership: &dyn Fn(&str, &ResourceLocation) -> bool,
    jigsaw_generator: &dyn StructureGenerator,
) -> BTreeMap<ResourceLocation, StructureStart>;

/// Context §D's `STRUCTURE_REFERENCES` flow. `neighbor_starts` is called for each of the
/// `17x17` neighborhood's chunks; the caller is responsible for having already generated
/// each neighbor to `STRUCTURE_STARTS` before invoking this (Context §D/§L, §M item 8).
pub fn scan_structure_references(
    chunk_x: i32, chunk_z: i32,
    neighbor_starts: &dyn Fn(i32, i32) -> BTreeMap<ResourceLocation, StructureStart>,
) -> BTreeMap<ResourceLocation, Vec<(i32, i32)>>;

/// Parses `structure.extra`'s jigsaw-specific `serde_json::Value` fields (Context §D).
pub struct JigsawExtra {
    pub start_pool: ResourceLocation,
    pub start_jigsaw_name: Option<String>,
    pub size: u32,
    pub start_height_absolute: Option<i32>,
    pub max_distance_from_center: u32,
    pub pool_aliases: Vec<crate::structure::jigsaw::PoolAliasBinding>,
    pub dimension_padding: (i32, i32),
    pub use_expansion_hack: bool,
    pub project_start_to_heightmap: bool,
}
pub fn parse_jigsaw_extra(extra: &BTreeMap<String, serde_json::Value>) -> Result<JigsawExtra, String>;
```

### `crates/worldgen/src/structure/jigsaw.rs` (new)

```rust
use crate::data::{BlockStateSpec, PoolElement, ResourceLocation, TemplatePool, TemplatePoolId};
use crate::random::RcRandomSource;
use crate::structure::generation::{BoundingBox, StructureGenerationOutcome, StructureGenerator, StructurePiece};
use crate::structure::template::StructureTemplate;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rotation { None, Cw90, Cw180, Ccw90 }
impl Rotation { pub fn all() -> [Rotation; 4]; }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mirror { None, LeftRight, FrontBack }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction4 { North, East, South, West }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JointType { Rollable, Aligned }

#[derive(Clone, Debug)]
pub struct JigsawBlockInfo {
    pub world_pos: [i32; 3],
    pub name: ResourceLocation,
    pub target: ResourceLocation,
    pub pool: ResourceLocation,
    pub final_state: BlockStateSpec,
    pub front: Direction4,
    pub top: Direction4,
    pub joint: JointType,
    pub placement_priority: i32,
    pub selection_priority: i32,
}

/// Context §F. `source.target == target.name`, front-opposite, top-match-unless-rollable —
/// asymmetric, `target`'s own `target` field is never consulted.
pub fn can_attach(source: &JigsawBlockInfo, target: &JigsawBlockInfo) -> bool;

/// Context §F — `Collections.shuffle`'s standard algorithm (public JDK spec, not
/// Mojang-specific): backward Fisher-Yates, `i` from `len-1` downto `1`.
pub fn collections_shuffle<T>(items: &mut Vec<T>, rng: &mut impl RcRandomSource);

/// Context §F. Weight-flattened `TemplatePool` — the literal-repetition list vanilla
/// itself builds, so a plain `next_int_bounded(len)` reproduces vanilla's own selection.
pub struct FlattenedPool<'a> { entries: Vec<&'a PoolElement>, fallback: TemplatePoolId }
impl<'a> FlattenedPool<'a> {
    pub fn build(pool: &'a TemplatePool) -> Self;
    pub fn get_random_template(&self, rng: &mut impl RcRandomSource) -> &'a PoolElement;
    pub fn all(&self) -> &[&'a PoolElement];
    pub fn get_max_size(&self) -> i32; // Context §F's expansion-hack cache
}

#[derive(Clone, Debug)]
pub struct JigsawJunction { pub source_x: i32, pub source_ground_y: i32, pub source_z: i32, pub delta_y: i32, pub projection: crate::data::Projection }
impl JigsawJunction { pub fn reversed(&self) -> JigsawJunction; }

#[derive(Clone, Debug)]
pub struct PoolElementRef {
    pub kind: PoolElementKind,
    pub location: Option<ResourceLocation>,   // single/legacy-single
    pub processors: Option<crate::data::ProcessorListId>,
    pub projection: crate::data::Projection,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoolElementKind { Single, LegacySingle, List, Feature, Empty }

#[derive(Clone, Debug)]
pub struct JigsawPieceData {
    pub element: PoolElementRef,
    pub rotation: Rotation,
    pub junctions: Vec<JigsawJunction>,
    pub ground_level_delta: i32,
}

/// Context §F. Resolved once per structure-start attempt.
#[derive(Clone, Debug)]
pub enum PoolAliasBinding {
    Direct { alias: ResourceLocation, target: ResourceLocation },
    Random { alias: ResourceLocation, targets: Vec<(ResourceLocation, u32)> },
    RandomGroup { groups: Vec<(Vec<PoolAliasBinding>, u32)> },
}

pub struct PoolAliasLookup(BTreeMap<ResourceLocation, ResourceLocation>);
impl PoolAliasLookup {
    pub fn create(bindings: &[PoolAliasBinding], structure_start_pos: [i32; 3], seed: i64) -> Self;
    pub fn lookup<'a>(&'a self, name: &'a ResourceLocation) -> &'a ResourceLocation;
}

/// Everything `add_pieces` needs, gathered by the caller (Context §F).
pub struct JigsawPlacementContext<'a> {
    pub world_seed: i64,
    pub template_pools: &'a BTreeMap<TemplatePoolId, TemplatePool>,
    pub template_pool_names: &'a BTreeMap<String, TemplatePoolId>,
    pub template_loader: &'a dyn crate::structure::template::TemplateSource,
    pub pool_aliases: &'a [PoolAliasBinding],
    pub heightmap: &'a dyn Fn(i32, i32) -> i32,
    pub start_pool: ResourceLocation,
    pub start_jigsaw_name: Option<String>,
    pub origin: [i32; 3],
    pub max_depth: u32,
    pub dimension_padding: (i32, i32),
    pub use_expansion_hack: bool,
    pub project_start_to_heightmap: bool,
}

/// Context §F's complete algorithm. `structure_seed_random` is caller-seeded per GEN-D6's
/// "structure generation itself" formula (`world_seed`, unsalted — M5-B01 §I).
pub fn add_pieces(
    ctx: &JigsawPlacementContext, structure_seed_random: &mut impl RcRandomSource,
) -> Option<Vec<StructurePiece>>;

/// `StructureGenerator` impl wrapping `add_pieces` + the biome check (Context §D/§F).
pub struct JigsawGenerator;
impl StructureGenerator for JigsawGenerator {
    fn find_generation_point(
        &self, structure: &crate::data::Structure, world_seed: i64, chunk_x: i32, chunk_z: i32,
        biome_at: &dyn Fn(i32, i32, i32) -> ResourceLocation,
        tag_membership: &dyn Fn(&str, &ResourceLocation) -> bool,
    ) -> StructureGenerationOutcome;
}
```

### `crates/worldgen/src/structure/template.rs` (new)

```rust
use crate::data::BlockStateSpec;

#[derive(Clone, Debug)]
pub struct TemplateBlockInfo { pub pos: [i32; 3], pub state: usize, pub nbt: Option<rc_nbt::owned::NbtCompound> }
#[derive(Clone, Debug)]
pub struct TemplateEntityInfo { pub pos: [f64; 3], pub block_pos: [i32; 3], pub nbt: rc_nbt::owned::NbtCompound }

/// Context §H's format.
#[derive(Clone, Debug)]
pub struct StructureTemplate {
    pub size: [i32; 3],
    pub palette: Vec<BlockStateSpec>,
    pub palettes: Option<Vec<Vec<BlockStateSpec>>>,
    pub blocks: Vec<TemplateBlockInfo>,
    pub entities: Vec<TemplateEntityInfo>,
}

#[derive(Debug, thiserror::Error)]
pub enum TemplateParseError {
    #[error("missing required field `{0}`")]
    MissingField(&'static str),
    #[error("malformed palette entry: {0}")]
    MalformedPalette(String),
}

pub fn parse_template(nbt: &rc_nbt::owned::NbtCompound) -> Result<StructureTemplate, TemplateParseError>;

/// Context §H — the operator-supplied loading seam. No implementation ships in this
/// blueprint's own test changeset beyond `DirectoryTemplateSource`/`CachingTemplateSource`;
/// every acceptance test constructs `StructureTemplate` values directly (own-made
/// fixtures, Constraints (c)).
pub trait TemplateSource {
    fn load(&self, location: &crate::data::ResourceLocation) -> Option<StructureTemplate>;
}

/// GEN-D23. `root` is an operator-configured directory (recommended default
/// `data/vanilla-structures/`, Context §H) laid out `<root>/<namespace>/<path>.nbt`.
pub struct DirectoryTemplateSource { pub root: std::path::PathBuf }
impl TemplateSource for DirectoryTemplateSource {
    fn load(&self, location: &crate::data::ResourceLocation) -> Option<StructureTemplate>;
}

pub struct CachingTemplateSource<S: TemplateSource> { inner: S, cache: std::cell::RefCell<std::collections::BTreeMap<crate::data::ResourceLocation, Option<StructureTemplate>>> }
impl<S: TemplateSource> CachingTemplateSource<S> {
    pub fn new(inner: S) -> Self;
}
impl<S: TemplateSource> TemplateSource for CachingTemplateSource<S> {
    fn load(&self, location: &crate::data::ResourceLocation) -> Option<StructureTemplate>;
}

#[derive(Clone, Debug)]
pub struct StructurePlaceSettings {
    pub rotation: crate::structure::jigsaw::Rotation,
    pub mirror: crate::structure::jigsaw::Mirror,
    pub offset: [i32; 3],
    pub processors: Vec<crate::data::StructureProcessor>,
    pub known_shape: bool,
    pub keep_jigsaws: bool,
}

pub struct PlaceOutcome {
    pub data_markers: Vec<(crate::data::ResourceLocation, [i32; 3])>,
    pub leftover_jigsaws: Vec<([i32; 3], BlockStateSpec)>,
}

/// Context §H's full `placeInWorld` sequence, including the processor pipeline (Context
/// §I) and liquid-reconciliation flood fill.
pub fn place_in_world(
    template: &StructureTemplate, settings: &StructurePlaceSettings, origin: [i32; 3],
    sink: &mut dyn crate::structure::generation::StructureBlockSink,
    block_resolver: &dyn rc_chunk_storage::BlockStateNames,
    barrier_state: crate::data::BlockStateSpec,
    reference_pos: [i32; 3],
) -> PlaceOutcome;
```

### `crates/worldgen/src/structure/processor.rs` (new)

```rust
use crate::data::{BlockStateSpec, PosRuleTest, RuleTest, StructureProcessor};
use crate::random::RcRandomSource;

#[derive(Clone, Debug)]
pub struct PlacedBlockInfo { pub pos: [i32; 3], pub state: BlockStateSpec, pub nbt: Option<rc_nbt::owned::NbtCompound> }

pub struct ProcessorContext<'a> {
    pub world_read: &'a dyn crate::structure::generation::StructureBlockSink,
    pub reference_pos: [i32; 3],
}

/// Context §I. One block's worth of `apply_processor` dispatch for all 11 kinds.
/// `None` cancels placement.
pub fn apply_processor(
    kind: &StructureProcessor, ctx: &ProcessorContext,
    template_pos: [i32; 3], world_pos: [i32; 3], info: PlacedBlockInfo,
) -> Option<PlacedBlockInfo>;

/// Context §I's `processBlockInfos` loop, including the fast/slow-path split for
/// `Capped`'s `evaluates_entire_piece_state`.
pub fn run_processor_list(
    processors: &[StructureProcessor], ctx: &ProcessorContext,
    blocks: Vec<PlacedBlockInfo>, chunk_bounds: Option<crate::structure::generation::BoundingBox>,
) -> Vec<PlacedBlockInfo>;

/// `Mth::get_seed`-seeded (M5-B01's own function). Context §I.
pub fn eval_rule_test(test: &RuleTest, template_block: &BlockStateSpec, world_block: &BlockStateSpec, rng: &mut impl RcRandomSource) -> bool;

pub fn eval_pos_rule_test(test: &PosRuleTest, template_pos: [i32; 3], world_pos: [i32; 3], reference_pos: [i32; 3]) -> bool;
```

### `crates/worldgen/src/structure/beardifier.rs` (new)

```rust
use crate::data::TerrainAdaptation;
use crate::structure::generation::BoundingBox;

pub const BEARD_KERNEL_RADIUS: i32 = 12;

#[derive(Clone, Copy, Debug)]
pub struct PieceBeardSource { pub bounding_box: BoundingBox, pub adaptation: TerrainAdaptation, pub ground_level_delta: i32 }

#[derive(Clone, Copy, Debug)]
pub struct JunctionBeardSource { pub x: i32, pub ground_y: i32, pub z: i32 }

/// Assembled once per chunk by the caller (a future GenStage-integration blueprint) from
/// `STRUCTURE_REFERENCES`'s own output, restricted to pieces/junctions within
/// `BEARD_KERNEL_RADIUS` of the chunk (Context §G).
pub struct BeardifierContext<'a> {
    pub pieces: &'a [PieceBeardSource],
    pub junctions: &'a [JunctionBeardSource],
}

/// Context §G — moderate confidence, flagged for GEN-D27 reconciliation.
pub fn sample_beardifier(ctx: &BeardifierContext, block_x: i32, block_y: i32, block_z: i32) -> f64;
```

### `crates/worldgen/src/structure/persistence.rs` (new)

Exactly the signatures given in Context §K.

### `crates/worldgen/src/density/interpreter.rs` (modify — M5-B03's own file)

- `evaluate_node`'s signature gains a trailing parameter: `beardifier: Option<&crate::structure::beardifier::BeardifierContext<'_>>`.
- Its `Beardifier{}` match arm becomes `beardifier.map(|b| crate::structure::beardifier::sample_beardifier(b, ctx.block_x, ctx.block_y, ctx.block_z)).unwrap_or(0.0)`.
- `DensityInterpreter<'a>` gains a field `beardifier: Option<&'a crate::structure::beardifier::BeardifierContext<'a>>`; `DensityInterpreter::new` gains a matching parameter; `DensityInterpreter::sample` forwards it to `evaluate_node`.

### `crates/worldgen/src/density/noise_chunk.rs` (modify — M5-B03's own file)

- `NoiseChunk<'a>` gains the identical `beardifier` field; `NoiseChunk::new` gains a matching parameter, forwarded to every internal `evaluate_node` call site.

### `crates/chunk-storage/src/chunk_nbt.rs` (modify — M2-B04's own file)

- `ChunkNbtDocument` gains a field `pub structures: rc_nbt::owned::NbtCompound` (opaque — Context §K).
- `ChunkNbtCodec::to_nbt` writes this field's value verbatim under the `structures` key, replacing the previously-hardcoded `{starts: {}, References: {}}` literal.
- `ChunkNbtCodec::from_nbt` captures the loaded `structures` compound into this field, replacing the previous discard-on-read behavior.
- `ChunkNbtDocument`'s existing "always writes/discards" documentation for the five fixed-default fields (M2-B04 Context) is updated to remove `structures` from that list — it is no longer a fixed-default field, it is a real (opaquely-typed) round-tripped one.

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary** (TEST-D45/D46, restated): every file under `crates/worldgen/src/structure/**` and the two `density/*.rs` modifications from Deliverables are committed **with public function bodies stubbed `todo!()`** (existing M5-B03 function bodies gain the new parameter, forwarding `todo!()` only for the new behavior path — the pre-existing, already-implemented non-beardifier logic is not touched by the test changeset), alongside every test file below. The `rc-chunk-storage` change (Deliverables' last item) is committed the same way. The follow-up implementation changeset fills in bodies and touches no test file, no fixture, and no file outside the paths named in Deliverables.

### `crates/worldgen/tests/structure_placement.rs`

1. `random_spread_matches_hand_derived_villages_vector` — `potential_structure_chunk(0, 0, 0, 34, 8, 10387312, SpreadType::Linear) == (15, 2)`; same for `(-5,-5) -> grid(-1,-1) -> (-9,-34)` and `(100,-37) -> grid(2,-2) -> (76,-60)` (Context §B's table, all three).
2. `random_spread_matches_hand_derived_ocean_monument_vector` — `potential_structure_chunk(12345, 0, 0, 32, 5, 10387313, SpreadType::Triangular) == (3, 19)`; `potential_structure_chunk(12345, 1, 1, 32, 5, 10387313, SpreadType::Triangular)` returns the **same** candidate (same grid cell).
3. `is_random_spread_chunk_true_only_at_the_candidate` — for the villages fixture, `is_random_spread_chunk(0, 15, 2, &villages_placement) == true`; `is_random_spread_chunk(0, 0, 0, &villages_placement) == false` (Context §D's own worked "does chunk (0,0) host the village" check).
4. `frequency_reduction_default_matches_hand_derived_nextfloat` — `passes_frequency_reduction(0, 0, 0, FrequencyReductionMethod::LegacyType2, 0.01) == false` (nextFloat `0.719376`, Context §B); at a coordinate this blueprint's own derivation pass confirms passes (any of the five listed, extended: this test additionally derives and asserts one passing coordinate at `frequency = 0.8`, e.g. `(0,0)`'s `0.719376 < 0.8`).
5. `weighted_draw_order_is_a_permutation_and_deterministic` — a 3-entry `StructureSelectionEntry` fixture (weights `2,3,5`); `weighted_draw_order(0, 0, 0, &entries)` called twice returns identical, length-3, index-`{0,1,2}`-permutation results both times.

### `crates/worldgen/tests/structure_concentric_rings.rs`

1. `generate_ring_positions_returns_exactly_count_entries` — a synthetic `RingBiomeSearch` mock that always returns `None` (raw point used unmodified); `generate_ring_positions(0, &stronghold_placement, &mock).len() == 128` (the vanilla stronghold `count`).
2. `generate_ring_positions_is_deterministic` — called twice with identical inputs, identical output.
3. `ring_positions_move_outward_across_rings` — the mean distance-from-origin of the first `spread=3` positions is strictly less than the mean distance of the last 3 positions (rings grow outward, Context §C's own algorithm shape — a structural, not golden-vector, assertion appropriate to this section's own flagged moderate confidence).

### `crates/worldgen/tests/structure_jigsaw_assembly.rs`

Own-made, hand-authored fixtures throughout (Constraints (c)) — no Mojang NBT. Every fixture is constructed specifically so the RNG bit stream never affects *which* candidate is chosen (single-candidate lists at every draw point), isolating the structural algorithm under test from needing a hand-traced LCG vector (Context §F's own stated rationale).

1. `single_piece_pool_terminates_immediately` — a `start` pool with one `EmptyPoolElement`; `add_pieces` returns `None` (Context §F: picking `EmptyPoolElement` as the center draw aborts immediately).
2. `two_piece_chain_attaches_at_compatible_jigsaw` — `start` pool: one `SinglePoolElement` (a hand-built 1x1x1 template, one jigsaw block at local `(0,0,0)`, `front=South`, `top=Aligned`? — `joint=Aligned`, `target="test:next_pool"`); `next` pool: one `SinglePoolElement` (a hand-built 1x1x1 template, one jigsaw block at local `(0,0,0)`, `front=North` [opposite of South], `top` matching, `name="test:next_pool"`), `fallback=minecraft:empty`. `add_pieces` with `max_depth=1` returns exactly 2 pieces; the second piece's `rotation == Rotation::None` and its position is offset by exactly the expected `[1,0,0]`-style delta the two jigsaw blocks' relative facing implies (hand-computed from the fixture's own geometry, not from any RNG trace).
3. `asymmetric_attach_rule_ignores_target_side_target_field` — two `JigsawBlockInfo` fixtures where `source.target == target.name` but `target.target != source.name`; `can_attach(&source, &target) == true` (Context §F/§M item 4 — the load-bearing asymmetry assertion).
4. `depth_cutoff_still_consults_fallback` — a `start` pool identical to test 2's, but `max_depth=0` for the **target**'s own further expansion (i.e. `ctx.max_depth=1` so the center piece is placed but its own child is placed at `depth==max_depth`, meaning the target pool's real candidates are skipped and only `fallback` is tried); construct `next`'s real pool with a candidate that would NOT satisfy `can_attach` (proving it was never even consulted) and `fallback` with one compatible `EmptyPoolElement`-terminated branch; assert the result still contains exactly 2 pieces (the center piece plus nothing further, since `EmptyPoolElement` contributes no additional piece) rather than panicking or silently producing 1.
5. `collections_shuffle_matches_hand_derived_vector` — `collections_shuffle(&mut vec![0,1,2,3], &mut RcLegacyRandom::new(0))` produces `[3, 0, 1, 2]` (Context §F's own hand-derived vector).
6. `flattened_pool_pick_matches_hand_derived_vector` — a pool with entries weight `50`/`1`; `FlattenedPool::build(&pool).get_random_template(&mut RcLegacyRandom::new(0))` selects the first (weight-`50`) entry (Context §F's own hand-derived `index=18` vector, which falls within the first 50 flattened slots).
7. `pool_alias_lookup_direct_binding` — one `PoolAliasBinding::Direct { alias: "a", target: "b" }`; `PoolAliasLookup::create(&[binding], [0,0,0], 0).lookup(&"a") == &"b"`; `.lookup(&"c") == &"c"` (unresolved passthrough).
8. `pool_alias_random_group_correlates_paired_bindings` — one `RandomGroup` with two weighted groups, each containing two `Direct`-style bindings sharing the same random draw within the chosen group; assert both aliases in the result resolve to the SAME group's pair (never a mismatched cross-group pair) — Context §F's trial-chambers-motivating property.

### `crates/worldgen/tests/structure_processors.rs`

1. `block_ignore_drops_matching_blocks` — `BlockIgnore { blocks: [air] }` applied to an air `PlacedBlockInfo` returns `None`; applied to a stone one returns `Some` unchanged.
2. `protected_blocks_cancels_on_existing_world_match` — a `ProcessorContext` whose `world_read` mock reports `stone` at the target position; `ProtectedBlocks { value: List([stone]) }` returns `None`; a mock reporting `air` returns `Some`.
3. `rule_processor_fires_only_when_both_predicates_pass` — one `ProcessorRule` with `input_predicate = BlockMatch(stone)`, `location_predicate = AlwaysTrue`, `output_state = dirt`; a template block of `stone` becomes `dirt`; a template block of `oak_log` (fails `input_predicate`) is unchanged.
4. `capped_processor_reports_evaluates_entire_piece_state` — `apply_processor`'s dispatch for `Capped { .. }` — a direct property assertion, not a full random-subset simulation (which would need an RNG trace this test deliberately avoids per test-file 3's own rationale) — confirms `run_processor_list` takes the whole-piece slow path whenever a `Capped` processor is present in the chain (constructed via a 2-element chunk-boundary fixture: one block inside `chunk_bounds`, one outside; both survive to the processor stage only when `Capped` is present, proving the fast-path clip was skipped).
5. `rule_test_random_block_match_uses_next_float` — `RandomBlockMatch { block: stone, probability: 1.0 }` against a matching block and a fixed `RcLegacyRandom::new(0)` always returns `true` (probability 1.0 — any `nextFloat() < 1.0` always holds, avoiding a hand-traced fractional vector); `probability: 0.0` always returns `false`.

### `crates/worldgen/tests/structure_beardifier.rs`

Structural-property tests only (Context §G's own moderate-confidence flag — no golden vectors claimed).

1. `beardifier_is_zero_far_from_any_source` — a `BeardifierContext` with one piece at the origin; `sample_beardifier` at a position `BEARD_KERNEL_RADIUS + 100` blocks away returns exactly `0.0`.
2. `beardifier_is_deterministic` — the same `(ctx, x, y, z)` sampled twice returns bit-identical results.
3. `encapsulate_and_none_contribute_nothing` — a piece with `adaptation: TerrainAdaptation::None` and one with `Encapsulate` both produce `sample_beardifier == 0.0` at every tested position (Context §G's own stated rule).

### `crates/worldgen/tests/structure_persistence.rs`

1. `pack_unpack_chunk_pos_round_trips` — `unpack_chunk_pos(pack_chunk_pos(x, z)) == (x, z)` for `(0,0)`, `(-1,-1)`, `(i32::MIN, i32::MAX)` (proptest-style bounded, or a small fixed table).
2. `encode_decode_structures_compound_round_trips` — a `StructureStart` fixture (one jigsaw piece, `references: 1`) plus one `References` entry; `decode_structures_compound(&encode_structures_compound(&starts, &references)) == (starts, references)`.
3. `invalid_start_omits_chunk_and_children_fields` — encoding a structure with no start recorded (this blueprint's own empty-map input) never emits an `"INVALID"`-id entry into `starts` at all (Context §K: `starts` only ever holds entries for structures that *do* have a start in this chunk — the `id == "INVALID"` convention describes vanilla's own historical placeholder-entry behavior, which this blueprint's own encoder does not reproduce since it never constructs a placeholder entry in the first place; this test asserts that non-behavior explicitly).
4. `max_references_cap_enforced` — constructing a `StructureStart` and incrementing its own `references` counter past `persistence::MAX_REFERENCES` (a small helper this test exercises directly) saturates at `1`, never exceeds it.

### `crates/chunk-storage/tests/chunk_nbt_structures_passthrough.rs`

1. `structures_field_round_trips_opaquely` — a non-empty synthetic `rc_nbt::owned::NbtCompound` (hand-built, unrelated to any real structure schema — this test only proves passthrough, not structure semantics) passed into `ChunkNbtDocument.structures`; `to_nbt` then `from_nbt` reproduces the identical compound.
2. `empty_structures_field_matches_prior_fixed_default_behavior` — an empty `structures` compound round-trips identically to M2-B04's own pre-existing `all_air_uninhabited_chunk` fixture's expectations (regression guard: confirms this blueprint's change does not alter that already-committed test's outcome).

## Implementation steps

1. **`Cargo.toml`.** Add the `rc-nbt` dependency line. Observable: `cargo metadata` resolves; `cargo build -p rc-worldgen` still fails only on `todo!()`.
2. **`structure/placement.rs`.** Implement `SpreadType::evaluate`, `potential_structure_chunk`, `is_random_spread_chunk`, `passes_frequency_reduction`, `violates_exclusion_zone`, `generate_ring_positions`, `weighted_draw_order`, `biome_set_contains` exactly per Context §B–§E. Observable: `structure_placement.rs` and `structure_concentric_rings.rs` tests pass.
3. **`structure/generation.rs`.** `BoundingBox` primitives; `StructureStart`/`StructurePiece`/`PieceKind` (forward-declared against `jigsaw.rs`'s `JigsawPieceData`, implemented after step 4); `StructureBlockSink`; `generate_structure_starts`/`scan_structure_references`/`dispatch_generator`/`parse_jigsaw_extra` per Context §D. Observable: compiles once `jigsaw.rs` exists (steps may interleave with step 4).
4. **`structure/jigsaw.rs`.** `Rotation`/`Mirror`/`Direction4`/`JointType`/`JigsawBlockInfo`/`can_attach`/`collections_shuffle`/`FlattenedPool`/`JigsawJunction`/`PoolElementRef`/`JigsawPieceData`/`PoolAliasBinding`/`PoolAliasLookup`/`JigsawPlacementContext`/`add_pieces`/`JigsawGenerator` exactly per Context §F. Observable: `structure_jigsaw_assembly.rs` tests pass.
5. **`structure/template.rs`.** `parse_template`, `DirectoryTemplateSource`, `CachingTemplateSource`, `StructurePlaceSettings`, `place_in_world` per Context §H (the liquid-reconciliation/shape-recalculation steps may be a minimal, always-no-op-for-known-shape-true implementation at this milestone's own scope, since `known_shape` is always `true` for jigsaw/pool placement per Context §H's own text — flag any narrower behavior explicitly in a doc comment rather than silently). Observable: compiles; no dedicated template-format test file is required by this blueprint's own Acceptance tests beyond what `structure_jigsaw_assembly.rs`'s fixtures already exercise via `StructureTemplate` literals.
6. **`structure/processor.rs`.** `apply_processor` (all 11 kinds), `run_processor_list`, `eval_rule_test`, `eval_pos_rule_test` per Context §I. Observable: `structure_processors.rs` tests pass.
7. **`structure/beardifier.rs`.** `sample_beardifier` per Context §G's own moderate-confidence formula. Observable: `structure_beardifier.rs` tests pass.
8. **`structure/persistence.rs`.** `pack_chunk_pos`/`unpack_chunk_pos`/`encode_structures_compound`/`decode_structures_compound` per Context §K. Observable: `structure_persistence.rs` tests pass.
9. **`structure/mod.rs`/`lib.rs`.** Wire every `pub mod`/`pub use` per Deliverables. Observable: `cargo build -p rc-worldgen` succeeds, zero `todo!()` remaining.
10. **`density/interpreter.rs` and `density/noise_chunk.rs`.** Apply the `evaluate_node`/`DensityInterpreter`/`NoiseChunk` modifications per Deliverables, passing `None` at every pre-existing call site that has no beardifier context available (preserving every one of M5-B03's own already-passing tests unchanged). Observable: `cargo nextest run -p rc-worldgen` — M5-B03's own full existing test suite still passes, unmodified.
11. **`rc-chunk-storage/src/chunk_nbt.rs`.** Apply the `ChunkNbtDocument`/`ChunkNbtCodec` modifications per Deliverables. Observable: `chunk_nbt_structures_passthrough.rs` tests pass; every one of M2-B04's own pre-existing round-trip tests still passes unmodified (test 2's own explicit regression guard).
12. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` — all four exit 0 for both `rc-worldgen` and `rc-chunk-storage`.

## Constraints & forbidden actions

(a) The implementation changeset (steps 1–12) never modifies any file under `crates/worldgen/tests/**` or `crates/chunk-storage/tests/**`, nor this document's own Acceptance tests section — those are committed first, verbatim, in the test changeset. (b) No new external `[workspace.dependencies]` entry; the one new dependency edge (`rc-worldgen -> rc-nbt`) is an existing internal crate, already in this workspace's dependency graph since M2-B02. (c) No Mojang or third-party reimplementation source is consulted or copied for this blueprint's own algorithms; every fixture used by `structure_jigsaw_assembly.rs` is a hand-authored, own-made `StructureTemplate`/`PoolElement`/pool construction, never a real vanilla `.nbt` file or a real vanilla template-pool JSON — GEN-D23's custody rule applies to test fixtures exactly as it applies to shipped code. `collections_shuffle`'s algorithm is restated as a public, textbook, JDK-specified CS fact (Context §F), not Mojang-specific expression. (d) GEN-D10's determinism discipline applies to every `f64` computation in `beardifier.rs` and anywhere `f32`/`f64` transcendentals appear (`generate_ring_positions`'s `angle.cos()`/`.sin()`): plain IEEE-754 operations only, never `mul_add`/FMA. (e) Every moderate-confidence formula flagged in Context (§B's `FrequencyReductionMethod` non-`LegacyType1` variants, §C's ring-loop assembly, §G's beardifier kernel, §K's jigsaw-specific NBT field names) is implemented exactly as this blueprint specifies — an implementer must not "improve" or silently reinterpret a flagged formula; any discrepancy found during a future GEN-D27 reconciliation pass is that pass's own correction to make, not a license for silent deviation now. (f) `rc-chunk-storage` never gains a dependency on `rc-worldgen`, in either direction beyond what already exists — the `structures` field stays opaque at that layer (Context §K, §M item 10).

## Verification commands

- `cargo build -p rc-worldgen` and `cargo build -p rc-chunk-storage` — zero warnings.
- `cargo nextest run -p rc-worldgen` — every test in `structure_placement.rs`, `structure_concentric_rings.rs`, `structure_jigsaw_assembly.rs`, `structure_processors.rs`, `structure_beardifier.rs`, `structure_persistence.rs`, plus M5-B03's own full pre-existing suite (unmodified, still green).
- `cargo nextest run -p rc-chunk-storage` — `chunk_nbt_structures_passthrough.rs`, plus M2-B04's own full pre-existing suite (unmodified, still green).
- `cargo test --doc -p rc-worldgen` / `-p rc-chunk-storage` — both exit 0.
- `cargo run -p xtask -- fmt-check` / `-- lint` / `-- lint-deps` — all exit 0.
- CI tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on `ubuntu-24.04` and `windows-2025`, clean checkout (TEST-D34/D37/D50).
