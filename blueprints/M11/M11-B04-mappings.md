# M11-B04 — Bedrock Mapping Data Pipeline (`rc-bedrock-mappings`)

| Field | Content |
|---|---|
| ID | M11-B04 |
| Milestone | M11 — Bedrock Cross-Play |
| Prerequisites | M0-B01 (workspace scaffold: root `Cargo.toml`, `xtask/`'s `Cli`/`Command` surface, `xtask/src/lib.rs` re-exports); M0-B07 (`xtask/src/datagen/{reports.rs, codegen.rs}`, `xtask/src/fixture_manifest.rs` — this blueprint reuses `reports::{RegistriesReport, RegistryReport, RegistryEntryReport, BlocksReport, BlockReport, BlockStateReport, find_default_state_id}` and `fixture_manifest::{build_manifest, verify_manifest, compute_sha256_hex, FixtureManifest, FixtureEntry, ManifestViolation}` directly, unmodified, rather than re-implementing either); M9-B05 (its `xtask` delta added `pub properties: BTreeMap<String, String>` to `reports::BlockStateReport` — this blueprint's own Java-side block-state property extraction depends on that field already existing; without it, the "hard part" §Context 4 algorithm has no source data) |
| Implements | CROSS-D2/D5 (crate placement, dependency-graph rules); CROSS-D19 (mapping-data sourcing, ASSET-D18(h) custody); CROSS-D20 (hand-authored spec + codegen generation pipeline); CROSS-D21 (regeneration trigger); CROSS-D15–D18 (translation-tier framework, applied here as the correspondence-classification vocabulary every table entry carries); CROSS-D6/D7 (pinned Bedrock version, bump process, restated as this crate's own versioning contract); CROSS-D4 (zero-cost-when-off, restated for this crate's own load-at-activation contract); WS-D2/WS-D5(e) (crate ratification, `crossplay` feature); ASSET-D30 (third-party firewall — GeyserMC mapping *outcomes* may inform the hand-authored spec, its generator code/output JSON never consulted outside the firewall); TEST-D47 (fixture integrity manifest, reused verbatim) |
| Crates touched | `crates/bedrock-mappings/` (new); `xtask/` (new `bedrock_datagen` module + `Command` variants, reusing M0-B07's `datagen::reports`/`fixture_manifest` modules); `xtask/Cargo.toml` (two additive lines, both already-pinned workspace deps — no root `Cargo.toml` edit) |
| Estimated scope | L (at the top of the range — six mapping categories plus a genuinely non-trivial bidirectional block-state algorithm; every category beyond blocks reuses blocks' machinery, keeping the total implementation surface tractable within one blueprint) |

## Goal & Done definition

Deliver `rc-bedrock-mappings` — the generated Java↔Bedrock correspondence-table crate CROSS-D2 names, analogous in role to `rc-registries` — plus the `xtask` generation pipeline that produces its committed content: `fetch-bedrock-data` (developer-local acquisition of Mojang's official Bedrock reference materials, EULA-gated, never committed, mirroring NET-D9/ASSET-D15's `server.jar` pattern) and `codegen-bedrock-mappings` (merges a hand-authored correspondence spec against extracted Bedrock facts *and* this project's own already-cached Java `--reports` data into generated Rust tables under `crates/bedrock-mappings/generated/<bedrock-protocol>-<java-protocol>/`). The crate's runtime surface is a set of compact, bidirectional lookup structures — one per mapping category (blocks, items, biomes, entities, sounds, particles) — assembled once by an explicit `MappingTables::load()` call that nothing in this crate or its generated data triggers automatically, so a `crossplay`-feature-compiled-but-config-disabled build performs zero mapping-table work (CROSS-D26).

This blueprint's own automated Done state does **not** require a real, legally-obtained Bedrock Dedicated Server distribution or `bedrock-samples` checkout — exactly as M0-B07's did not require a real `server.jar` for its own CI gate. Every pure-function piece (the spec-merge algorithm, the block-state property translation, the reverse-dictionary collision tie-break, fallback substitution, manifest build/verify) is proven against small, hand-authored synthetic fixtures. Populating the *full* correspondence spec against real Bedrock materials is a separate, ongoing, manually-performed editorial task (§Constraints (h)) — this blueprint delivers the pipeline and a minimal, high-confidence starter spec, not a claim of exhaustive vanilla coverage.

Done when:

- [ ] `cargo build -p rc-bedrock-mappings` and `cargo build -p xtask --all-features` both succeed with zero warnings.
- [ ] Every test in this blueprint's test changeset (`crates/bedrock-mappings/tests/{block_lookup.rs, item_lookup.rs, flat_category_lookup.rs, fallback_determinism.rs}`, `xtask/tests/bedrock_datagen_{spec_parsing,extract,codegen,cli_parsing}.rs`) passes under `cargo nextest run -p rc-bedrock-mappings -p xtask`, using only synthetic/in-memory fixtures — no real BDS distribution, no `bedrock-samples` checkout, no network access.
- [ ] `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps` all exit 0 — `lint-deps` in particular confirms `rc-bedrock-mappings` depends on exactly `rc-core` and `rc-registries` (CROSS-D5 rule 7), nothing else.
- [ ] Separately, and not part of this blueprint's own CI-checkable gate: given a locally-supplied, legally-obtained Bedrock Dedicated Server distribution and a local `Mojang/bedrock-samples` checkout for the pinned Bedrock version (CROSS-D6), plus an already-completed `cargo xtask fetch-data 26.2` run (M0-B07), `cargo xtask fetch-bedrock-data 26.44` followed by `cargo xtask codegen-bedrock-mappings` produces `crates/bedrock-mappings/generated/2168-776/{blocks.rs, items.rs, biomes.rs, entities.rs, sounds.rs, particles.rs, MANIFEST.json}`, and `cargo xtask verify-bedrock-mappings` exits 0 against that output. This is the manually-performed step that turns the starter spec's illustrative entries into the crate's real committed content.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` for this blueprint's entire automated changeset (TEST-D37/TEST-D43).

## Context (self-contained)

### 1. Crate placement and dependency rules — restated exactly

CROSS-D2: `rc-bedrock-mappings` (`crates/bedrock-mappings/`) holds "generated Java↔Bedrock block/item/biome/entity ID and property correspondence tables... analogous in role to `rc-registries`." CROSS-D5 rule 7 fixes its dependency graph precisely: **`rc-bedrock-mappings` depends only on `rc-core` and `rc-registries`** — mirroring "`rc-registries`' own 'graph's root leaf, plus one hop' shape." It has no dependency on `rc-bedrock-protocol`, `rc-bedrock-raknet`, `rc-bedrock-auth`, or `rc-bedrock-translator` — CROSS-D5 rule 5 shows the dependency arrow running the *other* way (`rc-bedrock-protocol → rc-bedrock-mappings`) — and, per WS-D3 rule 2 extended by CROSS-D5, it never depends on `rc-scheduler` or `rc-mechanics`. WS-D5(e): `rc-bedrock-mappings` is one of five `optional = true` dependencies of `rusty-clanker-server`, unified under the `crossplay` Cargo feature, on by default in the officially-distributed binary, strippable via `--no-default-features`. **This blueprint does not touch `crates/server/`** — wiring `rc-bedrock-mappings` behind `crossplay` in `rusty-clanker-server`'s own manifest, and the one call site that invokes `MappingTables::load()` at config-driven activation, belongs to a sibling M11 blueprint (the translator/composition-root work) and is named as a Needs-from item in Interfaces below.

**Scope boundary this dependency rule enforces, concretely:** because this crate cannot depend on anything that knows about *dynamic, in-flight* item stacks or entity instances (those types live in `rc-mechanics`, permanently off-limits), everything this crate produces is **static, type-level identity correspondence** — "this Java block state corresponds to this Bedrock block state," never "translate this specific player's held item stack's NBT." The latter — actual runtime item-stack/entity-instance data transcoding — is `rc-bedrock-translator`'s job, consuming this crate's tables as one input among several. This blueprint's item table therefore classifies *how well* an item's static identity crosses editions (§Context 5), not the mechanics of translating a live stack.

### 2. Sourcing and custody — restated exactly (CROSS-D19, ASSET-D18(h), ASSET-D30)

CROSS-D19: two Mojang-official sources, both developer/CI-local only, never committed raw. **(a) The official Bedrock Dedicated Server (BDS)** distribution (`minecraft.net/en-us/download/server/bedrock`, EULA-gated per its own download terms) for the pinned Bedrock version (CROSS-D6, currently 26.44 / protocol 2168) — the authoritative source for the exact set of valid block-state combinations (the *block palette*, §Context 4) and, historically, per-block/item legacy numeric ids where Mojang still publishes them. **(b) `Mojang/bedrock-samples`** (GitHub, official, versioned per release) — vanilla `behavior_pack`/`resource_pack` definition JSON for biomes, entities, sounds, and particles. **A third source this blueprint's pipeline draws on that CROSS-D19's text folds into "our own Java datagen": the already-cached output of the existing `xtask fetch-data`/`--reports` pipeline (NET-D9, M0-B07)** — `datagen-output/26.2/generated/reports/{registries.json, blocks.json}` — the Java-side half of every correspondence this crate builds. This blueprint's own `fetch-bedrock-data` verb therefore fetches *only* the Bedrock-side materials; it never re-fetches or re-derives Java facts, reusing M0-B07's cached output (or erroring, naming `cargo xtask fetch-data 26.2` as the fix, if that output is absent).

Custody (ASSET-D18(h)/ASSET-D19, extending ASSET-D13/D16's rule symmetrically to Bedrock, unchanged in substance):

| Artifact | May be committed? | Why |
|---|---|---|
| BDS distribution (zip/tar), raw extracted `block_palette.nbt`/behavior-pack JSON, the `bedrock-samples` checkout itself | **Never.** Cached under `.gitignore`d `oracle/bedrock/<version>/` and `oracle/bedrock-samples/<version>/` only. | ASSET-D13/D16: Mojang's own distributed binary/pack content is never shipped, exactly as `server.jar` and its assets never are. |
| `crates/bedrock-mappings/spec/*.ron` (hand-authored correspondence spec) | **Yes, committed.** | CROSS-D20: this project's own original editorial text — block/item/biome/entity/sound/particle correspondence choices, informed by observation and by GeyserMC's publicly documented mapping *outcomes* (ASSET-D18(e)), never its generator code or output JSON (ASSET-D30 firewall, CROSS-D29) — is authored content, not Mojang's. |
| `crates/bedrock-mappings/generated/<bedrock-protocol>-<java-protocol>/*.rs` | **Yes, committed.** | Same "functional fact vs. Mojang creative expression" test ASSET-D15/D25/D28(i) already apply to Java-side `--reports` data: a block/item/biome/entity/sound/particle's registered name and the property values a state legally holds are measurements of Mojang's own registration, not authored prose or art. |
| `crates/bedrock-mappings/generated/.../MANIFEST.json` | **Yes, committed.** | TEST-D47: derived, non-creative provenance metadata, same category as the `.rs` files it describes. |

### 3. The six mapping categories and what "correspondence" means per category (CROSS-D20)

CROSS-D20, restated: "many Java↔Bedrock block/item/biome/entity IDs correspond directly (a mechanical, factual mapping); others have no 1:1 counterpart and require an editorial correspondence choice (near-equivalent substitution, or an explicit 'no Bedrock equivalent' marker...)." Every category's spec file (`crates/bedrock-mappings/spec/{blocks,items,biomes,entities,sounds,particles}.ron`) therefore expresses each entry as one of three `CorrespondenceKind` values, restated once here and reused identically across every category:

```rust
pub enum CorrespondenceKind {
    /// Mechanical, direct correspondence — the two editions' concepts are the same thing.
    Exact,
    /// An editorial substitution: no exact counterpart exists, but a visually/functionally
    /// close Bedrock concept was chosen by hand. Always carries a `note` explaining the choice.
    NearEquivalent { note: &'static str },
    /// No reasonable Bedrock counterpart exists at all. The category's declared fallback is
    /// used in its place (§Context 6). Always carries a `note`.
    Unmapped { note: &'static str },
}
```

Two structurally different shapes follow from this, split along one line: **must every Java entry produce *some* Bedrock representation, or may it legitimately produce none?**

- **Blocks, items, biomes, entities — total.** A block occupies a coordinate; an item sits in a slot; a chunk column has a biome; a spawned entity is a real, positioned thing. Every one of these must be shown to a Bedrock client as *something*, so their tables are **total functions**: `Unmapped` entries resolve to the category's declared fallback (§Context 6), never to "nothing."
- **Sounds, particles — partial, by CROSS-D16(d)'s own design.** "A subset of particles have no Bedrock-native rendering equivalent and are **omitted** from what a Bedrock client sees, never from authoritative state" — omission is the *correct*, already-decided behavior here, not a gap to paper over with a fallback. Their tables therefore return `Option<BedrockId>`, and `None` is a first-class, intended result the translator is expected to simply not send anything for.

### 4. The hard part: block-state property mapping, both directions

**Java's shape (already generated, reused as-is).** A Java block state is `rc_registries::generated_v776::block_states::BlockStateId(pub u32)` — one of 32366 flat ids (`docs/research/mc-26.2/07-blocks-blockstates.md`). Its owning block name and property map are already available via `rc_registries::generated_v776::block_state_properties::{StateDescriptor, STATE_PROPERTIES, describe}` (M9-B05): `describe(id) -> Option<&'static StateDescriptor>` where `StateDescriptor { block: &'static str, properties: &'static [(&'static str, &'static str)] }` — both name and every value already in their serialized string form. This blueprint's codegen tool does **not** call this compiled runtime API (that would mean compiling and reflecting into `rc-registries` from inside `xtask`); it reads the identical facts directly from the same source `rc_registries` was generated from — M0-B07's cached `datagen-output/26.2/generated/reports/blocks.json`, via `reports::BlocksReport`/`BlockReport`/`BlockStateReport` (reused unmodified, Prerequisites), whose `properties: BTreeMap<String, String>` field (M9-B05's addition) carries the same per-state property map.

**Bedrock's shape (extracted from the block palette, CROSS-D19(a)).** Publicly documented (`wiki.bedrock.dev`'s block-states reference), confidence noted where the exact on-disk detail has not been re-verified against a live 26.44 BDS checkout at blueprint-writing time: a Bedrock block state is a `{name: string, states: compound, version: int}` triple — `name` a namespaced identifier (`"minecraft:oak_door"`), `states` a compound of *typed* name→value pairs (booleans as byte 0/1, small enumerations as either an int or a string token depending on the property — the exact tag-kind-per-property-name mapping is read directly off each extracted entry, never assumed), `version` an opaque per-entry format-revision marker BDS itself stamps, copied through verbatim and never edited by hand. The full, authoritative list of every legal `{name, states}` combination for the pinned Bedrock version ships as a binary-NBT block palette (`block_palette.nbt`, historically written by BDS itself on first run, or present in an add-on-authoring reference form — **confidence-flagged: confirm the exact filename/location against the actual downloaded BDS distribution at first real pipeline run**, per ASSET-D18(h)'s local-consultation allowance; if a future firewall pass ever needs GeyserMC's own extraction tooling to cross-check this, that pass is named in Open Questions). This crate models an extracted entry as:

```rust
/// One row of the extracted Bedrock block palette (CROSS-D19(a)) — codegen-internal, never
/// part of `rc-bedrock-mappings`' own committed runtime API (§Deliverables owns the runtime
/// shape; this is the raw fact xtask's codegen consumes to build it).
pub struct BedrockPaletteEntry {
    pub name: String,
    /// Sorted by property name — canonical order, fixed once here, never re-derived per call site.
    pub states: std::collections::BTreeMap<String, BedrockPropertyValueOwned>,
    pub version: i32,
}
pub enum BedrockPropertyValueOwned { Bool(bool), Int(i32), Str(String) }
```

**The correspondence spec's per-block entry (`crates/bedrock-mappings/spec/blocks.ron`).** One entry per Java block *name* (not per state — properties are handled once per block, applied to every one of that block's states):

```ron
BlockCorrespondence(
    java_block: "stone",
    kind: Exact,
    bedrock_block: "minecraft:stone",
    // No properties on either side — the common case for simple blocks.
    property_map: [],
)
BlockCorrespondence(
    java_block: "oak_door",
    kind: Exact,
    bedrock_block: "minecraft:wooden_door",
    property_map: [
        PropertyMapping(
            java_property: "facing",
            bedrock_property: "direction",
            // Java's 4-way string enum -> Bedrock's 0..3 int enum. Every Java value MUST
            // appear as a key (codegen fails otherwise, §Implementation step 4).
            value_map: { "east": "0", "south": "1", "west": "2", "north": "3" },
        ),
        PropertyMapping(
            java_property: "half",
            bedrock_property: "upper_block_bit",
            value_map: { "lower": "0", "upper": "1" },
        ),
        PropertyMapping(
            java_property: "open",
            bedrock_property: "open_bit",
            value_map: { "false": "0", "true": "1" },
        ),
        // "hinge" and "powered" have no Bedrock counterpart for this block on the wire — omitted
        // from property_map entirely, which is itself the "no Bedrock counterpart for this one
        // property" declaration (distinct from the whole-block Unmapped case above).
    ],
)
```

**Forward algorithm — Java state → Bedrock state (total, run once per Java block state at codegen time, never at runtime).**

1. Look up the java state's owning block name (`StateDescriptor.block`/`BlockStateReport`'s key, namespace-stripped) in the spec's block table.
2. If absent or `kind: Unmapped`: emit the category's declared fallback block state (§Context 6) for this Java state id; record the id in a `codegen`-time diagnostic list (surfaced in the generation report, §Implementation step 6) — never a silent substitution.
3. Otherwise: start from `bedrock_block` with an empty property compound. For each `PropertyMapping` in `property_map`, read `java_property`'s value out of this specific state's property map (from `BlockStateReport.properties`); if present, look it up in `value_map` — **a lookup miss here (a java value with no entry in `value_map`) is a fatal codegen error**, naming the block, property, and unmapped value (never silently dropped or defaulted, since an incomplete `value_map` is an authoring bug in the spec, not a legitimate "no Bedrock equivalent" case — that case is expressed by omitting the whole `PropertyMapping`, not by a partial `value_map`); insert the translated `(bedrock_property, value)` pair, typed per that property's kind as recorded in the Bedrock palette extraction (§Context 4's `BedrockPropertyValueOwned`). Any Java property *not* named in `property_map` is simply not carried across (the declared "no Bedrock distinction for this property" case).
4. **Validate against the extracted palette (CROSS-D19(a)):** the assembled `{name, states}` must exactly match one real entry in the extracted `BedrockPaletteEntry` set. If it does not, codegen fails with a fatal error naming the Java block/state and the constructed-but-invalid Bedrock state — this is the mechanical guard against ever shipping a mapping the real Bedrock client would reject as unrecognized. On a match, the matched entry's `version` is copied into the emitted `BedrockBlockState` (never hand-set in the spec — it is purely an extracted fact).

**Reverse algorithm — Bedrock state → Java state (best-effort, precomputed as a dictionary, never a second hand-written inverse).** Built once, at codegen time, by running the forward algorithm over **every** real Java block state (all 32366, enumerated from the cached `BlocksReport`) and inverting the resulting map:

1. For each Java `BlockStateId`, compute its `BedrockBlockState` per the forward algorithm above.
2. Group by the resulting `BedrockBlockState` (multiple Java states legitimately collapse onto one Bedrock state whenever a Java property has no `PropertyMapping` entry — Bedrock is coarser for that block). Insert every group into `bedrock_to_java` keyed by `BedrockBlockState`.
3. **Deterministic tie-break for a collapsed group of size > 1:** the Java state flagged `"default": true` in the group (`find_default_state_id`, reused unmodified from M0-B07) wins if it is a member of the group; otherwise the group member whose property map, serialized as `(name, value)` pairs sorted by name then by value string, sorts lexicographically smallest wins. This rule is total, needs no human judgment, and is exactly what makes two codegen runs over identical input produce a byte-identical reverse table (§Context 9).
4. Every fallback-substituted Java state from step "Unmapped" above is **excluded** from this reverse-dictionary construction — a Bedrock client's own fallback state never round-trips back to a specific Java block; the translator's own logic (out of scope here) decides what a genuinely-unrecognized Bedrock state means, if one is ever received.

This reverse-by-enumeration approach is deliberately the *only* place the Bedrock→Java direction is specified: there is no independent "reverse algorithm" to keep in sync with the forward one, because there structurally cannot be a drift between a function and its own precomputed inverse.

### 5. Items: the data-component vs. Bedrock-NBT divergence — restated stance

Java 26.2 items carry per-stack metadata as **data components** (`minecraft:damage`, `minecraft:custom_name`, `minecraft:enchantments`, …, post-1.20.5's model, superseding legacy NBT tags). Bedrock, per public documentation, has not made the equivalent migration for vanilla item-stack instance data — a Bedrock `ItemStack`'s per-instance metadata remains NBT-tag-shaped (`tag: {Damage: int, ench: [...], display: {Name, Lore}, ...}`, structurally closer to Java's own pre-1.20.5 shape). **This crate's stance, restated as policy rather than solved as an algorithm:** the item table this blueprint delivers is a **type-level identity table only** (Java item registry id ↔ Bedrock item name, mirroring blocks' simple, propertyless case — items have no per-state property system the way blocks do). Per-instance component↔NBT transcoding of a live stack is explicitly **out of this crate's scope**, deferred to `rc-bedrock-translator`'s own blueprint (a Needs-from item, Interfaces below), because that transcoding needs the actual component *type* definitions, which live in `rc-mechanics` — permanently unreachable from this crate (§Context 1). What this crate *does* provide is a coarse divergence classification per item, so the translator knows, without re-deriving it, how much runtime work a given item needs:

```rust
pub enum ItemComponentDivergence {
    /// The item's full functional identity survives with no component-level translation
    /// needed beyond what the translator's own generic NBT<->component bridge already does
    /// for every item (e.g. a plain, unenchanted, undamaged tool).
    FullyMapped,
    /// At least one of this item's commonly-set components has no Bedrock NBT-tag counterpart
    /// (documented per-item in the spec's `note`) — functions authoritatively server-side,
    /// degrades on the Bedrock client per CROSS-D16's tier discipline.
    PartiallyMapped { note: &'static str },
    /// No reasonable Bedrock item exists at all — the category fallback (§Context 6) is used.
    Unmapped { note: &'static str },
}
```

### 6. Unmappable-content handling: the fallback strategy, restated per category

Every *total* category (§Context 3) declares exactly one fallback value in its spec file, used whenever `CorrespondenceKind::Unmapped` applies:

| Category | Fallback mechanism | Rationale |
|---|---|---|
| Blocks | A single, spec-declared placeholder Bedrock block state (a real, vanilla, already-in-the-palette block chosen for being visually inert/clearly-a-placeholder — the exact vanilla identifier is confirmed against the real extracted palette at first real generation run, **confidence-flagged**, never invented ahead of that confirmation) | CROSS-D20's "explicit 'no Bedrock equivalent' marker" — a coordinate must show *something*; an honestly-inert placeholder is safer than a misleading visual substitute. |
| Items | A single, spec-declared placeholder Bedrock item (same confirm-against-real-data discipline as blocks) | Same reasoning — a slot must contain *something*. |
| Biomes | A single, spec-declared placeholder Bedrock biome (e.g. the closest generic "plains"-shaped vanilla biome) | Every loaded chunk column needs *a* biome for lighting/ambient-sound purposes on the Bedrock side. |
| Entities | A single, spec-declared placeholder Bedrock entity identifier | Expected to be near-empty in practice for M11's scope, since both editions ship the same "Chaos Cubed" content generation (CROSS-D6) — retained as a defensive completeness measure, not an anticipated common case. |
| Sounds | `None` (CROSS-D16(d) — omission is correct, not a gap) | Restated in §Context 3. |
| Particles | `None` (CROSS-D16(d)) | Restated in §Context 3. |

Every `Unmapped`/`NearEquivalent` spec entry's `note` field is **mandatory, non-empty** (codegen fails otherwise) — this is what keeps the eventual, fully-populated spec an honest, reviewable degradation list per CROSS-D18's "no silent drift" discipline, rather than a bare substitution table with no recorded reasoning.

### 7. Runtime representation and the zero-cost-when-off contract (CROSS-D4/D26)

`MappingTables` (§Deliverables) is a **plain struct with no ambient global state** — no `static`, no `OnceLock`, no `lazy_static`-style construct anywhere in this crate triggers table construction on first use from an always-reachable code path. The sole entry point is `MappingTables::load() -> Result<MappingTables, MappingLoadError>`, an ordinary function a caller invokes explicitly. This crate's own generated data (`generated::<version>::{blocks, items, ...}` modules) is plain `pub static` array/const data — free to exist in the compiled binary's read-only data section at zero runtime cost, exactly like `rc-registries`' own generated tables — but *indexing/hashing that data into the compact lookup structures* only happens inside `load()`'s body, which nothing calls unconditionally.

This satisfies CROSS-D26's two-part zero-cost claim precisely: (i) when the `crossplay` Cargo feature is off, this crate is not even compiled into `rusty-clanker-server` (WS-D5(e)) — no code, no data, period; (ii) when the feature is on but `[crossplay]` is absent or `enabled = false` (CROSS-D4's default), the crate *is* linked but `load()` is never called — the only cost is the generated tables' own static-data footprint in the binary image, never touched at runtime, never allocated, never hashed. Whichever sibling M11 blueprint owns config-driven activation is the sole caller of `load()`, invoked exactly once, gated on `[crossplay] enabled = true` — flagged as a Needs-from item (Interfaces).

Per-category compact structures (full signatures in Deliverables): each *total* category stores its Java→Bedrock direction as a flat, index-by-`u32`-id array (`Box<[T]>`, O(1) lookup, as compact as the data itself — no hashing needed since Java ids are already dense small integers) and its Bedrock→Java direction as a `std::collections::HashMap<BedrockKey, JavaId>` (Bedrock identities are structured, not small dense integers, so a hash map is the natural compact structure there). *Partial* categories (sounds, particles) use the identical array-for-forward/hashmap-for-reverse shape, just with `Option<BedrockId>` array elements instead of a mandatory value.

### 8. Versioning: the Bedrock-pin bump process, restated for this crate (CROSS-D6/D7/D21)

CROSS-D6 pins Bedrock Edition **protocol 2168** (Bedrock 26.44); NET-D1 pins Java **protocol 776** (26.2). This crate's generated output directory is named `crates/bedrock-mappings/generated/2168-776/` — literally `<bedrock-protocol>-<java-protocol>`, matching CROSS-D19/D20's own mermaid-diagram path exactly. CROSS-D21, restated exactly: "the mapping pipeline... re-runs whenever **either** edition's pin moves — a Java bump (NET-D1/NET-D2) or a Bedrock bump (CROSS-D6/D7) each independently trigger a full `fetch-bedrock-data`/`codegen-bedrock-mappings` re-run and a hand-review pass over the correspondence spec for any newly introduced block/item/biome/entity on either side." This blueprint gives that trigger a concrete, CI-checkable mechanism: `xtask verify-bedrock-mappings-version` (§Deliverables) fails if `crates/bedrock-mappings/generated/`'s only subdirectory name does not match the two pins currently hardcoded as `xtask`'s own defaults (`2168`/`776`) — a stale directory name after either doc's pin changes is caught mechanically, the same "mechanism, not discipline" pattern ASSET-D24's release-artifact scan already applies elsewhere. CROSS-D7's own bump gate (step (a): "the mapping-data pipeline re-run against the new Bedrock release") is exactly this blueprint's `fetch-bedrock-data`/`codegen-bedrock-mappings` pair, invoked with the new version's arguments.

### 9. Determinism — the concrete rule (mirrors M0-B07's, restated and extended)

Same four rules M0-B07 already established for the Java-only registry pipeline, extended to this pipeline's two-source merge: (1) parse every JSON/RON object into `BTreeMap`, never `HashMap`; (2) every generated list is sorted by an explicit, named key (Java protocol_id for the forward direction; the §Context 4 tie-break rule for the reverse direction) — never left to incidental map/text order; (3) no wall-clock timestamp, hostname, or run-varying value anywhere in generated output; (4) identifier sanitization (reusing M0-B07's `sanitize_mod_name`/`sanitize_const_name`/`is_rust_keyword` helpers verbatim — not reimplemented a second time) is a pure function of the input string alone. Given these four, `generate(java_facts, bedrock_facts, spec)` (Deliverables) is a pure function — proven by this blueprint's own `output_is_independent_of_input_insertion_order`-style test (§Acceptance tests).

## Deliverables

### `crates/bedrock-mappings/Cargo.toml`

```toml
[package]
name = "rc-bedrock-mappings"
version.workspace = true
edition.workspace = true
license = "MIT OR Apache-2.0"

[dependencies]
rc-core = { path = "../core" }
rc-registries = { path = "../registries" }
```

(No other dependency — CROSS-D5 rule 7. No `serde`/`ron`: the shipped crate never parses RON or JSON at runtime, only the compiled generated Rust `xtask` produces.)

### `crates/bedrock-mappings/src/ids.rs`

```rust
//! Runtime identity types shared by every mapping category. CROSS-D20/§Context 3-4.

/// One correspondence entry's provenance/quality classification — restated identically
/// across every category (§Context 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrespondenceKind {
    Exact,
    NearEquivalent { note: &'static str },
    Unmapped { note: &'static str },
}

/// A Bedrock property value's typed wire kind (§Context 4) — `Eq`/`Hash` derived so
/// `BedrockBlockState` itself can key a `HashMap` with no separate hashable-key wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BedrockPropertyValue {
    Bool(bool),
    Int(i32),
    Str(&'static str),
}

/// A full Bedrock block state — `{name, states, version}` per §Context 4. `states` is sorted
/// by property name at generation time (Context §9 rule 2) — never re-sorted at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BedrockBlockState {
    pub name: &'static str,
    pub states: &'static [(&'static str, BedrockPropertyValue)],
    pub version: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BedrockItemId {
    pub name: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemComponentDivergence {
    FullyMapped,
    PartiallyMapped { note: &'static str },
    Unmapped { note: &'static str },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BedrockBiomeId(pub &'static str);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BedrockEntityId(pub &'static str);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BedrockSoundId(pub &'static str);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BedrockParticleId(pub &'static str);
```

### `crates/bedrock-mappings/src/tables.rs`

```rust
//! Compact bidirectional lookup structures (§Context 7) and the sole `load()` entry point.

use crate::ids::*;
use rc_registries::generated_v776::block_states::BlockStateId;
use rc_registries::generated_v776::registries::RegistryEntryId;
use std::collections::HashMap;

#[derive(Debug)]
pub enum MappingLoadError {
    /// A generated table's array length does not match `rc_registries`' own known id-space
    /// size for that category — signals a version-pin mismatch between the compiled
    /// `rc-bedrock-mappings` generated data and the compiled `rc-registries` generated data
    /// (i.e. `crates/bedrock-mappings/generated/*` was generated against a different Java
    /// protocol pin than `crates/registries/generated/*` currently is). Never expected in a
    /// consistent checkout; caught here rather than silently producing an out-of-range panic
    /// later at an arbitrary lookup call site.
    IdSpaceSizeMismatch { category: &'static str, expected: usize, found: usize },
}

pub struct BlockMappings {
    java_to_bedrock: Box<[BedrockBlockState]>,
    java_to_bedrock_kind: Box<[CorrespondenceKind]>,
    bedrock_to_java: HashMap<BedrockBlockState, BlockStateId>,
}
impl BlockMappings {
    /// Total — always returns a real, palette-valid Bedrock block state (§Context 4 step 4's
    /// validation guarantee), the declared fallback for any `id` classified `Unmapped`.
    pub fn java_to_bedrock(&self, id: BlockStateId) -> BedrockBlockState {
        self.java_to_bedrock[id.0 as usize]
    }
    pub fn correspondence(&self, id: BlockStateId) -> CorrespondenceKind {
        self.java_to_bedrock_kind[id.0 as usize]
    }
    /// `None` only if `state` was never one this table's forward direction ever produced —
    /// i.e. a Bedrock peer sent a state outside what we ourselves declared reachable.
    pub fn bedrock_to_java(&self, state: &BedrockBlockState) -> Option<BlockStateId> {
        self.bedrock_to_java.get(state).copied()
    }
}

pub struct ItemMappings {
    java_to_bedrock: Box<[(BedrockItemId, ItemComponentDivergence)]>,
    bedrock_to_java: HashMap<BedrockItemId, RegistryEntryId>,
}
impl ItemMappings {
    pub fn java_to_bedrock(&self, id: RegistryEntryId) -> (BedrockItemId, ItemComponentDivergence);
    pub fn bedrock_to_java(&self, id: &BedrockItemId) -> Option<RegistryEntryId>;
}

/// Shared shape for the three simplest total categories — biomes and entities have no
/// per-state property system (§Context 4 is blocks-only), so one generic struct serves all
/// three rather than three near-identical hand-written copies.
pub struct FlatMappings<BedrockKey: std::hash::Hash + Eq + Copy> {
    java_to_bedrock: Box<[BedrockKey]>,
    bedrock_to_java: HashMap<BedrockKey, RegistryEntryId>,
}
impl<BedrockKey: std::hash::Hash + Eq + Copy> FlatMappings<BedrockKey> {
    pub fn java_to_bedrock(&self, id: RegistryEntryId) -> BedrockKey {
        self.java_to_bedrock[id.0 as usize]
    }
    pub fn bedrock_to_java(&self, key: &BedrockKey) -> Option<RegistryEntryId> {
        self.bedrock_to_java.get(key).copied()
    }
}
pub type BiomeMappings = FlatMappings<BedrockBiomeId>;
pub type EntityMappings = FlatMappings<BedrockEntityId>;

/// Partial categories (§Context 3) — forward direction may legitimately be `None`.
pub struct OptionalMappings<BedrockKey: std::hash::Hash + Eq + Copy> {
    java_to_bedrock: Box<[Option<BedrockKey>]>,
    bedrock_to_java: HashMap<BedrockKey, RegistryEntryId>,
}
impl<BedrockKey: std::hash::Hash + Eq + Copy> OptionalMappings<BedrockKey> {
    pub fn java_to_bedrock(&self, id: RegistryEntryId) -> Option<BedrockKey> {
        self.java_to_bedrock[id.0 as usize]
    }
    pub fn bedrock_to_java(&self, key: &BedrockKey) -> Option<RegistryEntryId> {
        self.bedrock_to_java.get(key).copied()
    }
}
pub type SoundMappings = OptionalMappings<BedrockSoundId>;
pub type ParticleMappings = OptionalMappings<BedrockParticleId>;

pub struct MappingTables {
    pub blocks: BlockMappings,
    pub items: ItemMappings,
    pub biomes: BiomeMappings,
    pub entities: EntityMappings,
    pub sounds: SoundMappings,
    pub particles: ParticleMappings,
    pub bedrock_protocol_version: u32,
    pub java_protocol_version: u32,
}
impl MappingTables {
    /// The sole entry point (§Context 7). Builds every `HashMap` from the compiled-in
    /// `generated::*` static data — cheap (a handful of linear passes over already-in-memory
    /// static arrays), but deliberately never called except by an explicit, config-gated
    /// caller (a sibling M11 blueprint's activation code) — never from a `static`/`OnceLock`
    /// reachable on any always-executed path.
    pub fn load() -> Result<MappingTables, MappingLoadError>;
}
```

### `crates/bedrock-mappings/src/lib.rs`

```rust
pub mod ids;
pub mod tables;

#[path = "../generated/2168-776/mod.rs"]
pub mod generated_v2168_776;
```

(Directory name `2168-776` matches CROSS-D19/D20's literal `<bedrock-protocol>-<java-protocol>` path; the Rust module identifier `generated_v2168_776` exists because Rust identifiers cannot contain `-` — the same kind of directory-name-vs-module-identifier gap `rc-registries`' own `generated/v776/` avoids simply by not needing a second number. Flagged explicitly here since it is the one naming wrinkle this crate's generated-output path introduces beyond WS-D13's existing precedent.)

### `crates/bedrock-mappings/generated/2168-776/mod.rs` (generated — shape only, content is `xtask codegen-bedrock-mappings`'s output)

```rust
//! Generated by `xtask codegen-bedrock-mappings` — do not edit by hand, re-run instead.
//! Bedrock protocol 2168 (26.44) <-> Java protocol 776 (26.2). CROSS-D19/D20.
pub mod blocks;
pub mod items;
pub mod biomes;
pub mod entities;
pub mod sounds;
pub mod particles;

pub const BEDROCK_PROTOCOL_VERSION: u32 = 2168;
pub const JAVA_PROTOCOL_VERSION: u32 = 776;
```

Each of the six submodules exposes exactly two `pub static` items following one shape (illustrated for `blocks.rs`; `items.rs`/`biomes.rs`/`entities.rs` drop the `_kind` array's per-property detail but keep the same two-array shape, `sounds.rs`/`particles.rs` use `Option<...>` elements per §Context 3):

```rust
// blocks.rs
use rc_bedrock_mappings::ids::{BedrockBlockState, CorrespondenceKind};
pub static JAVA_TO_BEDROCK: &[BedrockBlockState] = &[ /* one entry per Java BlockStateId, ascending */ ];
pub static JAVA_TO_BEDROCK_KIND: &[CorrespondenceKind] = &[ /* parallel array, same order */ ];
```

`tables::MappingTables::load()` reads `JAVA_TO_BEDROCK`/`JAVA_TO_BEDROCK_KIND` into `BlockMappings::java_to_bedrock`/`java_to_bedrock_kind` directly (`Box<[_]>::from(slice)`), and builds `bedrock_to_java` by iterating `JAVA_TO_BEDROCK` once, inserting `(state, BlockStateId(index as u32))` — the reverse map's *content* was already collision-resolved at codegen time (§Context 4's reverse algorithm), so `load()` itself performs no tie-breaking, only a single linear insert pass.

### `xtask/Cargo.toml` (modify — two additive lines, both already-pinned workspace deps)

```toml
[dependencies]
# ...existing lines (clap, xshell, serde, serde_json, sha1, sha2, thiserror, reqwest — unchanged)
ron      = { workspace = true }   # crates/bedrock-mappings/spec/*.ron parsing, CROSS-D20
simdnbt  = { workspace = true }   # block_palette.nbt extraction, CROSS-D19(a)
```

### `xtask/src/bedrock_datagen/spec.rs`

```rust
//! Parsed shape of `crates/bedrock-mappings/spec/*.ron` (§Context 3-6). One `#[derive(Deserialize)]`
//! type per category; `ron` reads these directly.

use std::collections::BTreeMap;

#[derive(serde::Deserialize, Debug, Clone)]
pub enum CorrespondenceKindSpec {
    Exact,
    NearEquivalent { note: String },
    Unmapped { note: String },
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct PropertyMapping {
    pub java_property: String,
    pub bedrock_property: String,
    pub value_map: BTreeMap<String, String>,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct BlockCorrespondence {
    pub java_block: String,
    pub kind: CorrespondenceKindSpec,
    /// Required even when `kind` is `Unmapped` (ignored in that case) — keeps the RON shape
    /// uniform; codegen never reads it for an `Unmapped` entry.
    pub bedrock_block: String,
    #[serde(default)]
    pub property_map: Vec<PropertyMapping>,
}

/// Shared shape for items/biomes/entities (no per-state properties — §Context 4 is blocks-only).
#[derive(serde::Deserialize, Debug, Clone)]
pub struct FlatCorrespondence {
    pub java_name: String,
    pub kind: CorrespondenceKindSpec,
    pub bedrock_name: String,
}

/// Sounds/particles — `bedrock_name` absent entirely when `kind` is `Unmapped` (§Context 3's
/// partial-category shape means there is no fallback to name).
#[derive(serde::Deserialize, Debug, Clone)]
pub struct OptionalCorrespondence {
    pub java_name: String,
    pub kind: CorrespondenceKindSpec,
    #[serde(default)]
    pub bedrock_name: Option<String>,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct BlockSpec {
    pub fallback: String,
    pub entries: Vec<BlockCorrespondence>,
}
#[derive(serde::Deserialize, Debug, Clone)]
pub struct FlatSpec {
    pub fallback: String,
    pub entries: Vec<FlatCorrespondence>,
}
#[derive(serde::Deserialize, Debug, Clone)]
pub struct OptionalSpec {
    pub entries: Vec<OptionalCorrespondence>,
}

/// Reads and parses one `spec/*.ron` file. `Err` names the file path and the `ron` parse error
/// verbatim (never swallowed).
pub fn read_ron<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Result<T, String>;

/// Validates a `BlockSpec`/`FlatSpec`: every entry's `note` (`NearEquivalent`/`Unmapped`) is
/// non-empty (§Context 6); every `PropertyMapping.value_map` is non-empty when present. Returns
/// every violation found (never stops at the first), each naming the offending `java_block`/
/// `java_name` and the specific problem.
pub fn validate_notes_nonempty(entries: &[BlockCorrespondence]) -> Vec<String>;
```

### `xtask/src/bedrock_datagen/extract.rs`

```rust
//! Extraction of Bedrock-side facts from downloaded materials (§Context 2/4). Pure, testable
//! functions taking already-read bytes — no filesystem/network access in this module itself
//! (that lives in `fetch.rs`).

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum BedrockPropertyValueOwned { Bool(bool), Int(i32), Str(String) }

#[derive(Debug, Clone, PartialEq)]
pub struct BedrockPaletteEntry {
    pub name: String,
    pub states: BTreeMap<String, BedrockPropertyValueOwned>,
    pub version: i32,
}

/// Parses an extracted `block_palette.nbt` (§Context 4, confidence-flagged shape) via
/// `simdnbt::borrow::read`: the root compound's top-level list (name confirmed against the
/// real file at first real run — read defensively: try every top-level list-valued key if the
/// expected one is absent, since this detail is unconfirmed) of compounds, each read as
/// `{name: string, states: compound, version: int}`. A `states` sub-tag's each entry is typed
/// per its own NBT tag id — `Byte` -> `Bool` (nonzero = true), `Int` -> `Int`, `String` -> `Str`;
/// any other tag kind for a states entry is a fatal parse error naming the block/property.
/// `Err` never panics on malformed input — this reads externally-sourced bytes.
pub fn parse_block_palette(nbt_bytes: &[u8]) -> Result<Vec<BedrockPaletteEntry>, String>;

/// A `bedrock-samples` behavior-pack entity definition JSON's one identifier field this
/// blueprint needs: `minecraft:entity.description.identifier`.
pub fn parse_entity_identifier(json_bytes: &[u8]) -> Result<String, String>;

/// A `bedrock-samples` biome definition JSON's identifier.
pub fn parse_biome_identifier(json_bytes: &[u8]) -> Result<String, String>;

/// A `bedrock-samples` particle-effect JSON's `particle_effect.description.identifier`.
pub fn parse_particle_identifier(json_bytes: &[u8]) -> Result<String, String>;

/// The sound-definitions JSON's top-level sound-event-name keys.
pub fn parse_sound_event_names(json_bytes: &[u8]) -> Result<Vec<String>, String>;

/// The full set of Bedrock-side facts this pipeline consumes, one field per category.
pub struct BedrockFacts {
    pub blocks: Vec<BedrockPaletteEntry>,
    pub items: Vec<String>,
    pub biomes: Vec<String>,
    pub entities: Vec<String>,
    pub sounds: Vec<String>,
    pub particles: Vec<String>,
}

/// Walks a downloaded `bedrock-samples` checkout directory plus a separately-extracted
/// `block_palette.nbt` path, calling the per-category parse functions above over every
/// matching file. Path patterns used (`behavior_pack/entities/*.json` etc., §Context 4) are
/// **confidence-flagged** — confirmed once against a real checkout at first pipeline run;
/// `Err` on a directory layout that does not match names the exact expected-vs-found path.
pub fn extract(bedrock_samples_dir: &std::path::Path, block_palette_path: &std::path::Path) -> Result<BedrockFacts, String>;
```

### `xtask/src/bedrock_datagen/codegen.rs`

```rust
//! Pure spec+facts -> generated-Rust transform (§Context 4/9), plus the CLI-facing `run`.

use super::extract::BedrockFacts;
use super::spec::{BlockSpec, FlatSpec, OptionalSpec};
use crate::datagen::reports::{BlocksReport, RegistriesReport};

pub const CODEGEN_TOOL_VERSION: &str = concat!("xtask-bedrock-codegen/", env!("CARGO_PKG_VERSION"));

pub struct GeneratedFiles {
    /// `(relative filename under generated/<bedrock-protocol>-<java-protocol>/, content)`, in
    /// write order: `mod.rs`, `blocks.rs`, `items.rs`, `biomes.rs`, `entities.rs`, `sounds.rs`,
    /// `particles.rs`.
    pub files: Vec<(String, String)>,
    /// One line per `Unmapped`/fallback-substituted Java entry across every category —
    /// surfaced to the operator running `codegen-bedrock-mappings`, never silently dropped
    /// (§Context 2 step "record... in a codegen-time diagnostic list").
    pub diagnostics: Vec<String>,
}

/// Pure transform. Deterministic per §Context 9's four rules. `java_blocks`/`java_registries`
/// are the already-cached, already-parsed `--reports` data (Prerequisites — M0-B07's types,
/// reused unmodified); `bedrock` is `extract::extract`'s output; `block_spec`/`item_spec`/
/// `biome_spec`/`entity_spec`/`sound_spec`/`particle_spec` are the parsed `spec/*.ron` files.
/// Implements §Context 4's forward+reverse block algorithm in full (the only category with a
/// property system); every other category reuses the same "look up in spec, validate against
/// extracted facts, fall back or omit per §Context 3/6" shape with no property step.
pub fn generate(
    java_blocks: &BlocksReport,
    java_registries: &RegistriesReport,
    bedrock: &BedrockFacts,
    block_spec: &BlockSpec,
    item_spec: &FlatSpec,
    biome_spec: &FlatSpec,
    entity_spec: &FlatSpec,
    sound_spec: &OptionalSpec,
    particle_spec: &OptionalSpec,
    bedrock_protocol_version: u32,
    java_protocol_version: u32,
) -> Result<GeneratedFiles, String>;

pub struct CodegenArgs {
    pub java_reports_dir: std::path::PathBuf,
    pub bedrock_samples_dir: std::path::PathBuf,
    pub block_palette_path: std::path::PathBuf,
    pub spec_dir: std::path::PathBuf,
    pub out_dir: std::path::PathBuf,
    pub bedrock_protocol_version: u32,
    pub java_protocol_version: u32,
    pub source_bds_sha1: String,
}

/// I/O wrapper: reads the Java `--reports` files (reusing `crate::datagen::reports`), reads
/// `spec_dir`'s six `*.ron` files (`spec::read_ron`, validated via `validate_notes_nonempty`),
/// calls `extract::extract`, calls `generate`, writes every file under `args.out_dir`, builds
/// and writes `MANIFEST.json` via `crate::fixture_manifest::build_manifest` (reused, unmodified
/// — Prerequisites), then self-checks via `crate::fixture_manifest::verify_manifest`, mirroring
/// M0-B07's `codegen::run` exactly. Prints every `GeneratedFiles::diagnostics` line before
/// returning `Ok`.
pub fn run(args: &CodegenArgs) -> Result<(), String>;
```

### `xtask/src/bedrock_datagen/fetch.rs`

```rust
//! CLI-facing I/O wrapper for `fetch-bedrock-data`. Downloads (or accepts locally-supplied
//! paths for) BDS and a `bedrock-samples` checkout (§Context 2). Reuses `reqwest`
//! (already an `xtask` dependency, M0-B08) for HTTP; no new download-mechanism crate.

pub struct FetchBedrockArgs {
    pub bedrock_version: String,
    /// Local BDS zip, bypassing download. **The exact BDS download URL/API is
    /// confidence-flagged (§Context 4)** — this flag exists specifically so this verb is
    /// usable the moment that URL is confirmed, without blocking this blueprint on it.
    pub bds_zip: Option<std::path::PathBuf>,
    /// Local `bedrock-samples` checkout directory, bypassing a git clone/archive download.
    pub bedrock_samples_dir: Option<std::path::PathBuf>,
    pub offline: bool,
}

pub struct FetchBedrockOutcome {
    pub bedrock_samples_dir: std::path::PathBuf,
    pub block_palette_path: std::path::PathBuf,
    pub bds_sha1: String,
}

/// `Err` cases mirror M0-B07's `fetch::run` pattern: `offline` without both local paths
/// supplied; a supplied path that does not exist; a download failure (network/HTTP error,
/// surfaced verbatim). Persists `bds_sha1` to a sidecar file exactly as M0-B07's
/// `fetch.rs` step 6g does, for a later, separate `codegen-bedrock-mappings` process
/// invocation to read.
pub fn run(args: &FetchBedrockArgs) -> Result<FetchBedrockOutcome, String>;
```

### `xtask/src/bedrock_datagen/mod.rs`

```rust
pub mod codegen;
pub mod extract;
pub mod fetch;
pub mod spec;
```

### `xtask/src/lib.rs` (modify — one additional re-export, alongside `datagen`/`fixture_manifest`)

```rust
pub mod bedrock_datagen;
```

### `xtask/src/main.rs` (modify — extend `Command`; existing variants unchanged, not repeated)

```rust
#[derive(clap::Subcommand, Debug, PartialEq)]
pub enum Command {
    // ...FmtCheck | Lint | LintDeps | Test | FetchData{..} | Codegen{..} | VerifyGenerated | ...
    // (every variant added by M0-B01/M0-B07/M0-B08 and subsequent blueprints, unchanged)

    /// CROSS-D19: acquire the pinned Bedrock version's official reference materials locally.
    FetchBedrockData {
        /// Bedrock version id, e.g. "26.44".
        bedrock_version: String,
        #[arg(long)]
        bds_zip: Option<std::path::PathBuf>,
        #[arg(long)]
        bedrock_samples_dir: Option<std::path::PathBuf>,
        #[arg(long)]
        offline: bool,
    },
    /// CROSS-D20: merge crates/bedrock-mappings/spec/*.ron against a prior FetchBedrockData
    /// run's extracted facts and this project's already-cached Java --reports data, emitting
    /// crates/bedrock-mappings/generated/<bedrock-protocol>-<java-protocol>/.
    CodegenBedrockMappings {
        #[arg(long, default_value = "26.2")]
        java_version: String,
        #[arg(long, default_value = "26.44")]
        bedrock_version: String,
        #[arg(long, default_value_t = 776)]
        java_protocol_version: u32,
        #[arg(long, default_value_t = 2168)]
        bedrock_protocol_version: u32,
    },
    /// TEST-D47: recompute crates/bedrock-mappings/generated/.../MANIFEST.json's hashes
    /// against the files on disk and fail on any mismatch.
    VerifyBedrockMappings,
    /// CROSS-D21: fail if crates/bedrock-mappings/generated/'s directory name does not match
    /// this xtask build's own current Bedrock/Java protocol-version defaults (§Context 8).
    VerifyBedrockMappingsVersion,
}
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary.** This blueprint's test changeset is exactly the eight files below, plus every `crates/bedrock-mappings/src/{ids.rs, tables.rs}` and `xtask/src/bedrock_datagen/{mod.rs, spec.rs, extract.rs, codegen.rs, fetch.rs}` function body stubbed `todo!()` (signatures/derives/doc comments unchanged, matching Deliverables verbatim), plus `crates/bedrock-mappings/{Cargo.toml, src/lib.rs}`, `xtask/{Cargo.toml, src/lib.rs, src/main.rs}` edits, plus a minimal `crates/bedrock-mappings/generated/2168-776/mod.rs` containing only empty `pub static` arrays (so the crate compiles before real codegen ever runs) and the six starter `crates/bedrock-mappings/spec/*.ron` files (§Constraints (h) — a handful of real, high-confidence entries: `stone`/`dirt`/`oak_planks`/`glass` for blocks with `Exact` correspondence and empty `property_map`, no `oak_door`-shaped property-mapping entry yet). The implementation changeset (Implementation steps) fills in real bodies only; it must not modify any of the eight test files, must not weaken their assertions, and must not touch `crates/bedrock-mappings/generated/2168-776/{blocks,items,...}.rs`'s real, full content (that is the separate manual step, §Constraints (h), outside both changesets).

### `crates/bedrock-mappings/tests/block_lookup.rs`

Constructs a tiny synthetic `BlockMappings` directly (a test-only constructor, `BlockMappings::from_raw(java_to_bedrock: Vec<BedrockBlockState>, java_to_bedrock_kind: Vec<CorrespondenceKind>) -> Self`, added to `tables.rs`'s Deliverables as `#[cfg(test)]`-gated or a `pub(crate)`/test-feature helper — named explicitly here since it is not otherwise in Deliverables' public surface) with 3 entries: one `Exact` two-property block state, one `Exact` zero-property block state, one `Unmapped` state pointing at a declared fallback.

1. `java_to_bedrock_returns_exact_state_for_mapped_id` — the two mapped ids each return their own distinct `BedrockBlockState`, not the fallback.
2. `java_to_bedrock_returns_fallback_for_unmapped_id` — the unmapped id returns exactly the fallback state used to construct the fixture.
3. `correspondence_reports_kind_correctly` — `correspondence()` returns `Exact` for the two mapped ids and `Unmapped{..}` for the third.
4. `bedrock_to_java_round_trips_for_mapped_states` — `bedrock_to_java(&java_to_bedrock(id)) == Some(id)` for both `Exact` ids (the "round-trip mapping properties, mappable set" acceptance requirement).
5. `bedrock_to_java_returns_none_for_a_state_never_produced` — a `BedrockBlockState` not equal to any of the fixture's three forward outputs returns `None`.

### `crates/bedrock-mappings/tests/item_lookup.rs`

Mirrors `block_lookup.rs`'s 5 cases for `ItemMappings` (using its own `from_raw` test constructor), substituting `ItemComponentDivergence` assertions for `CorrespondenceKind` ones on the `PartiallyMapped`/`Unmapped` fixture entries.

### `crates/bedrock-mappings/tests/flat_category_lookup.rs`

1. `flat_mappings_round_trips_for_mapped_entries` — `FlatMappings<BedrockBiomeId>` (representative of biomes/entities) constructed with 3 synthetic entries; round-trip holds for the two `Exact` entries.
2. `flat_mappings_fallback_for_unmapped` — the third entry's fallback substitution matches.
3. `optional_mappings_returns_none_for_partial_category` — `OptionalMappings<BedrockSoundId>` (representative of sounds/particles) constructed with one entry that has no Bedrock counterpart; `java_to_bedrock()` returns `None`, not a fallback — asserts §Context 3's total-vs-partial distinction is real, not accidentally collapsed to one shape.

### `crates/bedrock-mappings/tests/fallback_determinism.rs`

1. `repeated_fallback_lookups_are_identical` — calling `java_to_bedrock` on the same unmapped id 100 times in a loop returns bit-identical `BedrockBlockState` values every time (no hidden randomness/iteration-order dependency in the lookup path itself).
2. `mapping_tables_load_produces_consistent_sizes` — a `MappingTables::load()` call against a *minimal but non-empty* synthetic `generated_v2168_776`-shaped test module (compiled under `#[cfg(test)]` inside this crate, not the real generated data) succeeds and every category's array length matches that test module's own declared length — proves `MappingLoadError::IdSpaceSizeMismatch` is wired correctly without needing the real, full-sized generated tables.

### `xtask/tests/bedrock_datagen_spec_parsing.rs`

1. `parses_block_spec_minimal` — `spec::read_ron::<BlockSpec>` on a literal matching Context §4's `stone`/`oak_door` RON excerpt; asserts both entries parse, `oak_door`'s `property_map` has 3 entries, `facing`'s `value_map` has exactly the 4 keys shown.
2. `validate_notes_nonempty_flags_missing_note` — a `BlockCorrespondence` with `kind: Unmapped { note: "" }`; `validate_notes_nonempty` returns exactly one violation naming that entry's `java_block`.
3. `validate_notes_nonempty_passes_on_well_formed_entries` — the same fixture with a non-empty note; returns an empty `Vec`.

### `xtask/tests/bedrock_datagen_extract.rs`

1. `parse_entity_identifier_reads_description_field` — a synthetic minimal JSON literal `{"minecraft:entity":{"description":{"identifier":"minecraft:zombie"}}}`; returns `"minecraft:zombie"`.
2. `parse_biome_identifier_and_particle_identifier_and_sound_event_names` — three more synthetic minimal literals, one per remaining `parse_*` function, each asserting the extracted value(s).
3. `parse_block_palette_reads_typed_states` — a synthetic, hand-built minimal NBT byte buffer (built via `simdnbt::owned` types + `.write()`, not hand-written raw bytes — mirroring `M2-B02`'s own NBT-layer test-construction convention) encoding one palette entry with a `Byte` states-tag, one `Int`, one `String`; asserts `parse_block_palette` returns one `BedrockPaletteEntry` with `states` containing `Bool`/`Int`/`Str` values matching the tag kinds used.
4. `parse_block_palette_rejects_unsupported_states_tag_kind` — the same construction with a states-tag of an unsupported kind (e.g. a `List`); asserts `Err` naming the offending property.

### `xtask/tests/bedrock_datagen_codegen.rs`

1. `generates_block_state_deterministically_from_forward_algorithm` — a 1-block synthetic `BlocksReport` (`oak_door`-shaped, 2 states differing only in a property with no `PropertyMapping` entry — the lossy-collapse case) plus a matching `BlockSpec`/extracted palette fixture (containing exactly the one resulting `BedrockPaletteEntry` both states must resolve to); `generate(...)`'s `blocks.rs` output assigns both Java states the same `BedrockBlockState`.
2. `reverse_table_tie_break_prefers_default_flagged_state` — same 2-state collapse fixture, with the *second* state (not the report's `"default": true`-flagged one) sorting lexicographically smaller under §Context 4 step 3's tie-break rule; asserts the generated reverse table nonetheless maps that collapsed `BedrockBlockState` back to the **default-flagged** state's id (proves the default-flag rule takes precedence over the lexicographic fallback, per the stated rule ordering).
3. `unmapped_block_falls_back_and_is_diagnosed` — a `BlockSpec` entry with `kind: Unmapped`; `generate(...)` succeeds, that Java state's forward entry equals the declared fallback, and `GeneratedFiles::diagnostics` contains exactly one line naming that Java block.
4. `codegen_fails_on_value_map_gap` — a `PropertyMapping` whose `value_map` is missing an entry for a value the fixture's `BlocksReport` actually uses; `generate(...)` returns `Err` naming the block/property/value.
5. `codegen_fails_when_constructed_state_absent_from_palette` — a `BlockSpec`/`BlocksReport` pair whose resulting `{name, states}` does not appear in the fixture's extracted palette entries; `Err` naming the Java block/state and the invalid constructed Bedrock state (§Context 4 step 4's validation guarantee).
6. `output_is_independent_of_input_insertion_order` — mirrors M0-B07's own test of the identical name: two logically-identical `(BlocksReport, RegistriesReport, BedrockFacts, ...)` tuples built via reverse-order `.insert()` calls; `generate(...)` produces byte-for-byte identical `GeneratedFiles::files`.
7. `flat_category_codegen_falls_back_and_round_trips` — a small synthetic `FlatSpec`/`RegistriesReport` pair for biomes (2 `Exact` entries, 1 `Unmapped`); asserts the generated `biomes.rs` shape matches §Context 6's fallback rule.
8. `optional_category_codegen_omits_unmapped_entries` — a small synthetic `OptionalSpec`/`RegistriesReport` pair for sounds (1 `Exact`, 1 `Unmapped` with no `bedrock_name`); asserts the generated `sounds.rs` array element for the unmapped entry is the `None` variant, never a fallback (§Context 3's partial-category contract, proven at the codegen level as well as the runtime level `flat_category_lookup.rs` proves it at).

### `xtask/tests/bedrock_datagen_cli_parsing.rs`

1. `parses_fetch_bedrock_data_with_version_only` — `Cli::try_parse_from(["xtask", "fetch-bedrock-data", "26.44"])` matches `Command::FetchBedrockData { bedrock_version, bds_zip: None, bedrock_samples_dir: None, offline: false } if bedrock_version == "26.44"`.
2. `parses_codegen_bedrock_mappings_with_defaults` — `["xtask", "codegen-bedrock-mappings"]` matches `Command::CodegenBedrockMappings { java_version, bedrock_version, java_protocol_version: 776, bedrock_protocol_version: 2168 } if java_version == "26.2" && bedrock_version == "26.44"`.
3. `parses_verify_bedrock_mappings` — `["xtask", "verify-bedrock-mappings"]` → `Command::VerifyBedrockMappings`.
4. `parses_verify_bedrock_mappings_version` — `["xtask", "verify-bedrock-mappings-version"]` → `Command::VerifyBedrockMappingsVersion`.

## Implementation steps

1. **`crates/bedrock-mappings/Cargo.toml`, `src/lib.rs`, `xtask/Cargo.toml`.** Apply the edits from Deliverables exactly. Observable: `cargo metadata` succeeds; both crates compile (with every function body still `todo!()`'d from the test changeset).
2. **`crates/bedrock-mappings/src/ids.rs`.** Plain data types, as shown — no logic to implement beyond the derives. Observable: compiles.
3. **`crates/bedrock-mappings/src/tables.rs`.** Implement `BlockMappings`/`ItemMappings`/`FlatMappings`/`OptionalMappings`'s methods exactly per their doc comments (array index / `HashMap::get` — no algorithm beyond what is already stated); implement the `#[cfg(test)]` `from_raw`-style constructors the test changeset's `block_lookup.rs`/`item_lookup.rs`/`flat_category_lookup.rs` need; implement `MappingTables::load()`: for each category, `Box::from(generated::CATEGORY::JAVA_TO_BEDROCK)` (and `_KIND` where present) for the forward array, then one linear pass building the `HashMap` (`.iter().enumerate()`, inserting `(value, IdType(index as u32))` — skip an entry if the category's fallback is being inserted a second time under a different index, so the reverse map's fallback key always points at the *first* Java id observed, deterministic since `generated::*` arrays are themselves already sorted). Validate array lengths against `rc_registries::generated_v776`'s own known counts (`block_states::BLOCK_STATE_COUNT` — already committed per M0-B07 — and `registries::<category>::COUNT` for items/biomes/entities/sounds/particles, each already emitted per M0-B07's `generate_registries_rs` algorithm's `pub const COUNT: u32 = {entries.len()}` line); mismatch → `Err(MappingLoadError::IdSpaceSizeMismatch{..})`. Populate `MappingTables::bedrock_protocol_version`/`java_protocol_version` directly from `generated_v2168_776::{BEDROCK_PROTOCOL_VERSION, JAVA_PROTOCOL_VERSION}` — copied straight through, never re-derived. Observable: `block_lookup.rs`, `item_lookup.rs`, `flat_category_lookup.rs`, `fallback_determinism.rs` all pass.
4. **`xtask/src/bedrock_datagen/spec.rs`.** `read_ron`: `std::fs::read_to_string` + `ron::from_str`, mapping both error kinds to `Err(format!("{}: {e}", path.display()))`. `validate_notes_nonempty`: iterate entries, push a message for any `NearEquivalent`/`Unmapped` whose `note.is_empty()`, and for any non-empty `property_map` entry whose `value_map.is_empty()`. Observable: `bedrock_datagen_spec_parsing.rs` passes.
5. **`xtask/src/bedrock_datagen/extract.rs` — the four `parse_*_identifier`/`parse_sound_event_names` functions.** Plain `serde_json::Value` navigation (`.get("minecraft:entity").and_then(|v| v.get("description")).and_then(|v| v.get("identifier")).and_then(|v| v.as_str())`, `Err` with a fixed descriptive message on any `None` in the chain) — confidence-flagged field paths per Deliverables' doc comments, implemented exactly as named there. `parse_block_palette`: `simdnbt::borrow::read(&mut std::io::Cursor::new(nbt_bytes))`; on `Nbt::None` or an `Err`, return `Err` naming "empty or malformed NBT document"; `.as_compound()`, then locate the top-level list per the doc comment's "try every top-level list-valued key" fallback (`compound.keys().find_map(|k| compound.list(k))`); for each list element's compound, read `name` (`.string("name")`), `states` (`.compound("states")`), `version` (`.int("version")`), each `Option::ok_or_else` into a named `Err` on absence; for each `states` entry, `tag.id()` dispatch: byte id → `Bool(tag.byte().unwrap() != 0)`, int id → `Int(tag.int().unwrap())`, string id → `Str(tag.string().unwrap().to_string())` (via `Mutf8Str`'s own string conversion), any other id → `Err` naming the property and its unsupported tag id. `extract`: `std::fs::read_dir` walks over the (confidence-flagged) expected subpaths, calling the per-file parse function on each match, collecting into `BedrockFacts`; a completely-absent expected subpath is a named `Err` (never a silently-empty `Vec`). Observable: `bedrock_datagen_extract.rs` passes.
6. **`xtask/src/bedrock_datagen/codegen.rs` — `generate`.** Implement exactly §Context 4/9's forward algorithm (steps 1–4), reverse-dictionary construction (steps 1–4), and the parallel simpler algorithm for items/biomes/entities (spec lookup → validate the chosen `bedrock_name` appears in the corresponding `BedrockFacts` field, `Err` naming the entry if not found → fallback substitution) and sounds/particles (identical minus the fallback step, `None` instead). Use M0-B07's already-committed `sanitize_mod_name`/`sanitize_const_name`/`is_rust_keyword`/`strip_namespace` helpers (imported from `crate::datagen::codegen`, made `pub(crate)` there if not already — a small, additive visibility change to that existing file, not a logic change, so it stays within this blueprint's own implementation changeset without touching M0-B07's own tests) for every generated Rust identifier this pipeline emits. Emit `mod.rs` per Deliverables' exact literal content (with the two version consts substituted from `bedrock_protocol_version`/`java_protocol_version` arguments). Observable: all 8 cases in `bedrock_datagen_codegen.rs` pass.
7. **`xtask/src/bedrock_datagen/codegen.rs` — `run`.** Mirrors M0-B07's `codegen::run` structure exactly: read the four input sources (Java reports via `crate::datagen::reports`, six RON specs via `spec::read_ron`+`validate_notes_nonempty` — any validation violation is a fatal `Err` before `generate` is ever called, extracted Bedrock facts via `extract::extract`), call `generate`, write every `GeneratedFiles::files` entry under `args.out_dir`, build+write `MANIFEST.json` via `crate::fixture_manifest::build_manifest`, self-check via `crate::fixture_manifest::verify_manifest`, print `diagnostics`. Observable: exercised by the manual verification procedure (needs real reports+spec+extracted-fact content on disk to be meaningful beyond what `generate`'s own tests already cover — no automated test, matching M0-B07's own `codegen::run` precedent).
8. **`xtask/src/bedrock_datagen/fetch.rs` — `run`.** Mirrors M0-B07's `fetch::run` structure (its `--server-jar`/`--offline` case-handling pattern, Prerequisites) applied to two local-or-downloaded inputs instead of one: `bds_zip` (download mechanism confidence-flagged — Implementation note: leave the actual download URL as a `todo!("confirm BDS download URL/API against minecraft.net at implementation time")` *only* inside the network-download branch, never inside the `--bds-zip`-supplied or `--offline` branches, so this verb is fully usable via locally-supplied materials the moment this blueprint lands, with only the convenience auto-download path pending that one confirmation — flagged explicitly in Constraints (i) as the one deliberately-incomplete code path this blueprint ships) and `bedrock_samples_dir` (a `git clone --depth 1 --branch <tag>` shelled out via `xshell`, tag-naming scheme likewise confidence-flagged with the identical `--bedrock-samples-dir`-supplied bypass). Persist `bds_sha1` to a sidecar file exactly as M0-B07 step 6g does. Observable: exercised by the manual verification procedure only (no automated test needs real network/BDS access — CLI parsing alone is covered by `bedrock_datagen_cli_parsing.rs`).
9. **`xtask/src/main.rs` dispatch.** `FetchBedrockData{..}` → `bedrock_datagen::fetch::run`. `CodegenBedrockMappings{..}` → resolve `java_reports_dir`/`bedrock_samples_dir`/`block_palette_path`/`spec_dir`/`out_dir` (the last two are fixed repo-relative paths, `crates/bedrock-mappings/{spec, generated/<bedrock_protocol_version>-<java_protocol_version>}`) → `bedrock_datagen::codegen::run`. `VerifyBedrockMappings` → `fixture_manifest::verify_manifest` against the hardcoded `crates/bedrock-mappings/generated/2168-776/MANIFEST.json`. `VerifyBedrockMappingsVersion` → `std::fs::read_dir("crates/bedrock-mappings/generated")`, assert exactly one subdirectory whose name equals `format!("{}-{}", DEFAULT_BEDROCK_PROTOCOL, DEFAULT_JAVA_PROTOCOL)` (two `xtask`-local `const`s, `2168`/`776`, doc-commented as "bump alongside CROSS-D6/NET-D1 — this is the mechanical trigger CROSS-D21 requires, §Context 8"); mismatch or absence → actionable `Err`.
10. **Run the automated test suite.** `cargo nextest run -p rc-bedrock-mappings -p xtask` — every new test file's cases pass, using only synthetic data; every prior `xtask`/`rc-bedrock-mappings`-adjacent test remains green, untouched.
11. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps` all still exit 0. `lint-deps` specifically confirms `rc-bedrock-mappings`'s dependency edges are exactly `rc-core`+`rc-registries` (CROSS-D5 rule 7) — a violation here is this blueprint's own bug, not a pre-existing condition.
12. **(Manual, requires legally-obtained Bedrock materials — not part of this blueprint's own CI-checkable Done state.)** Whoever has legal access to a BDS distribution and a `bedrock-samples` checkout for Bedrock 26.44, with `cargo xtask fetch-data 26.2` already run (M0-B07), runs `cargo xtask fetch-bedrock-data 26.44` then `cargo xtask codegen-bedrock-mappings`, confirms `crates/bedrock-mappings/generated/2168-776/*` now exist with real (not empty-placeholder) content, runs `cargo xtask verify-bedrock-mappings` and `cargo xtask verify-bedrock-mappings-version` (both expect exit 0), and confirms `cargo build -p rc-bedrock-mappings` still succeeds against the real generated data. The resulting files are committed in their own changeset (§Context 2's custody table).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding**, exactly as stated under Acceptance tests above — the eight test files are committed first with every referenced function body `todo!()`-stubbed; the implementation changeset fills in bodies only, never edits a test file, never touches the real (non-placeholder) content of `crates/bedrock-mappings/generated/2168-776/{blocks,items,biomes,entities,sounds,particles}.rs`.

(b) **No new external dependencies beyond the pinned set.** `rc-bedrock-mappings` itself adds none (only internal `rc-core`/`rc-registries` path deps, CROSS-D5 rule 7). `xtask` adds exactly `ron`/`simdnbt`, both already present in `[workspace.dependencies]` at the versions `12-workspace-structure.md` already pins — no version override, no new crate. Do not add `anyhow`, a third-party RakNet/Bedrock-protocol crate, or `azalea`-adjacent tooling — none of that belongs to this blueprint's scope (translation/transport are sibling blueprints').

(c) **No Mojang or third-party reimplementation code.** Every format this blueprint parses is derived from `wiki.bedrock.dev`'s public documentation (ASSET-D18(b)) and this project's own already-cached, already-legally-obtained `--reports` data (ASSET-D18(a), reused via M0-B07). GeyserMC's/CloudburstMC's/`Sandertv/gophertunnel`'s own mapping-generator source or output JSON is never consulted while writing any file this blueprint creates or in authoring `spec/*.ron`'s starter content — only their publicly documented mapping *outcomes* may inform an editorial choice (ASSET-D18(e)), and their code specifically is off-limits to this blueprint's own implementer role under the ASSET-D30 firewall (CROSS-D29) — a deeper look at one specific tricky case, if ever needed, is a designated-researcher pass into `docs/research/third-party/`, never this blueprint's own work.

(d) **The forward/reverse block-state algorithm (§Context 4, restated in Implementation step 6) is binding, not a suggestion** — an implementer must not substitute a different tie-break rule, a different fallback-selection order, or a hand-written independent reverse algorithm; `bedrock_datagen_codegen.rs`'s acceptance tests assert on this exact algorithm's observable output.

(e) **Never embed a wall-clock timestamp, hostname, or any other run-varying value in generated output** (§Context 9) — no field for it exists in any Deliverables struct, and none may be added.

(f) **Runtime item-stack/entity-instance NBT-vs-component transcoding stays out of scope**, per §Context 1/5 — this crate's item/entity tables are type-level identity only; do not add per-instance translation logic, and do not add a dependency on `rc-mechanics` or any component-type-carrying crate to make such logic possible (WS-D3 rule 2/CROSS-D5 rule 7 forbid it outright regardless).

(g) **`crates/server/` stays untouched.** Wiring `rc-bedrock-mappings` behind the `crossplay` feature in `rusty-clanker-server`'s own manifest, and the call site invoking `MappingTables::load()`, belong to a sibling M11 blueprint (Interfaces' Needs-from item) — this blueprint delivers the library and its generation pipeline only.

(h) **The starter `spec/*.ron` content is deliberately minimal and high-confidence-only** (a handful of propertyless, unambiguous `Exact` block entries — `stone`/`dirt`/`oak_planks`/`glass`-class blocks whose Java-and-Bedrock identity has been stable and identical by name for the entire history of both editions). Populating the full, ~1196-Java-block-and-proportionate-item/biome/entity/sound/particle correspondence spec — including every property-mapping entry, every editorial `NearEquivalent`/`Unmapped` judgment call, and confirmation of every confidence-flagged extraction path/URL/tag-naming-scheme this blueprint names — is explicit, ongoing, out-of-this-blueprint's-automated-Done-state editorial work, ratified incrementally by whoever performs Implementation step 12 and every later CROSS-D21-triggered regeneration. This blueprint's own Done state is the *pipeline*, proven correct on synthetic data, plus that minimal honest starting point — never a claim of exhaustive vanilla coverage.

(i) **The BDS-zip auto-download branch inside `fetch.rs`'s `run` is the one deliberately-incomplete code path this blueprint ships** (Implementation step 8) — gated behind a `todo!()` with a citation to the exact confirmation needed, reachable only when neither `--bds-zip` nor `--offline` is supplied. This does not block this blueprint's own Done state (the automated tests never exercise that branch; Implementation step 12's manual procedure always supplies `--bds-zip` explicitly) but must not be silently completed with an invented, unverified URL — closing it is a small, separate, explicitly-flagged follow-up once the real download mechanism is confirmed (Open Questions).

(j) **No `unsafe` code.** Every function in this blueprint's deliverables is implementable in 100% safe Rust, including the NBT parsing (`simdnbt`'s own public API is already safe).

## Verification commands

Automated, run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43) — no BDS distribution, no `bedrock-samples` checkout, no network access:

```
cargo build -p rc-bedrock-mappings
cargo build -p xtask --all-features
cargo nextest run -p rc-bedrock-mappings -p xtask
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
```

Expected: every command exits 0. `cargo nextest run -p rc-bedrock-mappings -p xtask` includes all eight of this blueprint's new test files (`block_lookup.rs`, `item_lookup.rs`, `flat_category_lookup.rs`, `fallback_determinism.rs`, `bedrock_datagen_spec_parsing.rs`, `bedrock_datagen_extract.rs`, `bedrock_datagen_codegen.rs`, `bedrock_datagen_cli_parsing.rs`), alongside every prior `rc-bedrock-mappings`/`xtask` test suite, all green.

Manual, requires a locally-supplied or fetchable legally-obtained Bedrock Dedicated Server distribution and `Mojang/bedrock-samples` checkout for Bedrock 26.44, plus a prior `cargo xtask fetch-data 26.2` run (NET-D9, never run by CI in this blueprint's own gate) — this blueprint's own real-data acceptance step:

```
cargo xtask fetch-bedrock-data 26.44 --bds-zip <path> --bedrock-samples-dir <path>
cargo xtask codegen-bedrock-mappings
cargo xtask verify-bedrock-mappings
cargo xtask verify-bedrock-mappings-version
cargo build -p rc-bedrock-mappings
```

Expected: every command exits 0; `crates/bedrock-mappings/generated/2168-776/{blocks.rs, items.rs, biomes.rs, entities.rs, sounds.rs, particles.rs, MANIFEST.json}` exist with real content and, once committed, are what all future ordinary CI runs build against — with no further Bedrock-materials or network dependency from that point on. CI green on both `ubuntu-24.04` and `windows-2025` for the automated portion above is this blueprint's own authoritative done-signal (TEST-D50); the manual portion's completion is confirmed once, by whoever performs it, and its result (the seven committed files) is what subsequent CI runs then verify automatically forever after.

## Interfaces

**Provides to `rc-bedrock-protocol`'s blueprint (CROSS-D5 rule 5):** the `BedrockBlockState`/`BedrockItemId`/`BedrockBiomeId`/`BedrockEntityId`/`BedrockSoundId`/`BedrockParticleId` types and `MappingTables`' lookup surface — the raw material `rc-bedrock-protocol` needs to build its own wire-level runtime-ID palette (StartGame's block-palette encoding) and any legacy numeric item-id assignment; this blueprint's block/item tables are name-and-property-keyed, never wire-numeric, by design (§Context 1).

**Provides to `rc-bedrock-translator`'s blueprint:** `MappingTables` as the static-identity input its per-connection translation logic consumes for every packet direction; `ItemComponentDivergence`/`CorrespondenceKind` as the pre-computed classification it reads rather than re-derives, so its own tier-handling logic (CROSS-D15–D18) never needs to re-decide "does this block/item have a Bedrock equivalent" from scratch.

**Needs from a sibling M11 composition/translator blueprint:** the one call site that invokes `MappingTables::load()` exactly once, gated on `[crossplay] enabled = true` (§Context 7) — this blueprint deliberately does not own or place that call, since it lives wherever crossplay's own runtime activation logic lands, outside `rc-bedrock-mappings`' dependency-graph reach.

**Needs from `12-workspace-structure.md`:** none — WS-D2/WS-D5(e) already ratify this crate and its dependency rules; this blueprint implements what is already decided, adding nothing new to that document.

## Open Questions

- The exact BDS download URL/API (Constraint (i)) and the exact `Mojang/bedrock-samples` tag-naming scheme for a given Bedrock release are confidence-flagged throughout this blueprint, pending confirmation against the real, current download surfaces at first real pipeline run (Implementation step 12) — not blocking this blueprint's own automated Done state, but the one concrete item a near-term follow-up commit should close.
- The exact on-disk `bedrock-samples` subpaths this blueprint's `extract::extract` walks (`behavior_pack/entities/*.json` and siblings, §Context 4/Deliverables) are likewise confidence-flagged against wiki.bedrock.dev's publicly documented add-on layout, not a live-verified checkout — confirmed at the same Implementation step 12 pass, updated as a small, reviewed follow-up if any path has moved by the time a real checkout is available.
- Whether a future revision needs a designated-researcher (ASSET-D30) pass into GeyserMC's own mapping-generator output specifically to cross-check one or more genuinely ambiguous block-state property translations this blueprint's own black-box/documentation-only research cannot resolve confidently — left open per CROSS-D20's own text ("routed through the ASSET-D30 firewall per CROSS-D29 if a maintainer ever needs deeper study of one specific tricky case"), not pre-emptively requested here since no such case has yet been identified against real data.
- The full, exhaustive correspondence spec (Constraint (h)) is explicitly unfinished by this blueprint and is expected to grow incrementally across the CROSS-D21 regeneration cadence, not as a single follow-up task.
