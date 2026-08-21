# M5-B02 — Worldgen Data Pipeline: Extraction, Schema & Compilation

| Field | Content |
|---|---|
| ID | M5-B02 |
| Milestone | M5 — World Generation Parity |
| Prerequisites | M0-B01 (workspace scaffold: `rc-worldgen` already exists as an empty-shell crate, `crates/worldgen/Cargo.toml` with normal deps `rc-core`, `rc-chunk-storage`, `rc-registries`, no external deps, no features yet); M0-B07 (`xtask`'s NET-D9 pipeline — this blueprint's own `fetch-worldgen-data` verb reuses `xtask/src/fetch_data.rs`'s `fetch_server_jar`/`run_data_reports` exactly as M0-B07's own `fetch.rs` does, and this blueprint's `compile-worldgen-data` verb reuses M0-B07's `xtask/src/fixture_manifest.rs` — `build_manifest`/`verify_manifest`/`compute_sha256_hex` — unmodified, the identical TEST-D47 manifest scheme, not a second implementation) |
| Implements | GEN-D7 (extraction mechanism), GEN-D8 (interpreter-over-JSON architecture — this blueprint is its concrete parsing/compilation half), GEN-D9 (compiled representation: `serde_json` parse → canonicalize → `postcard`-encode → `include_bytes!`/`OnceLock` load), GEN-D12 (density-function node enumeration, re-verified against the pinned 26.2 target per its own "not assumed unchanged" instruction — corrections below), GEN-D13 (noise router's fixed 15-slot shape), GEN-D14 (climate-parameter quantization — resolves 04's Open Question with a confirmed constant), GEN-D17 (surface-rule tree shape), GEN-D19 (placement-modifier chain shape), GEN-D21 (structure-set/template-pool/processor-list graph shape), GEN-D23/GEN-D24 (custody split, restated and applied to this blueprint's own artifacts), ASSET-D16/ASSET-D25 (binding confirmation of that split), WS-D13 (generated-output-homes convention, extended to `rc-worldgen`'s own generated directory), TEST-D45/D46/D47/D50 (test-first changeset boundary, fixture manifest, CI-is-authoritative — restated for this blueprint's artifacts) |
| Crates touched | `xtask/` (new `worldgen_data` module + two new CLI verbs); `rc-worldgen` (`crates/worldgen/`, new `data` module + its own `generated/v776/` output directory, WS-D13-pattern) |
| Estimated scope | L |

## Goal & Done definition

Give `xtask` two new verbs, `fetch-worldgen-data <version>` and `compile-worldgen-data`, that together turn vanilla's worldgen datapack JSON into a validated, id-interned, immutable compiled graph committed as `crates/worldgen/generated/v776/data.postcard` (GEN-D9), and give `rc-worldgen` the Rust types plus a loader (`data::load()`) that deserialize that blob once at process startup. This blueprint owns **parsing, validation, reference resolution, and graph shape only** — it defines the schema every worldgen JSON node family decodes into and the compiled, id-interned graph that schema canonicalizes to; it does **not** implement density-function evaluation, noise sampling, surface-rule execution, feature placement, or structure assembly (all of that is a later M5 blueprint's `sample`/`evaluate`/`place` methods built on top of the types this blueprint defines).

Done when:

- [ ] `cargo build -p xtask --all-features` and `cargo build -p rc-worldgen --all-features` both succeed with zero warnings.
- [ ] Every acceptance test in this blueprint's own test changeset passes under `cargo nextest run -p xtask` and `cargo nextest run -p rc-worldgen`, using only synthetic fixture JSON — no real `server.jar`, no network access, no local Java installation required to go green.
- [ ] `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` all still exit 0.
- [ ] Separately, and not part of this blueprint's own CI-checkable gate (mirrors M0-B07's Acceptance Criterion 3 pattern exactly): given a locally supplied, legally obtained Minecraft 26.2 `server.jar` and a local Java 25+ runtime, `cargo xtask fetch-worldgen-data 26.2` followed by `cargo xtask compile-worldgen-data` produces `crates/worldgen/generated/v776/{data.postcard, MANIFEST.json}`; `cargo xtask verify-generated` (M0-B07's existing verb, which this blueprint extends to also check this new manifest — Implementation steps) exits 0.
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test`) green on both `ubuntu-24.04` and `windows-2025` (TEST-D34/D37/D43) for this blueprint's entire automated changeset, on a clean checkout (TEST-D50).

## Context (self-contained)

### Scope boundary, restated precisely

GEN-D8: "Rusty Clanker implements a data-driven interpreter over [vanilla worldgen JSON], not a hand-ported reimplementation... Only the interpreter's fixed, small set of primitive node-kind implementations... is native Rust code; every composition graph, numeric constant, spacing/salt value, spline knot, climate parameter point, surface-rule condition tree, and feature-placement ordering is read from the extracted JSON... with zero hand-transcription." This blueprint builds the **graph**, not the **primitives**: it defines the Rust types every JSON node family decodes into, resolves every named cross-reference into a stable integer id, validates the result (unknown node kinds, dangling references, reference cycles), and serializes it deterministically. A later blueprint (owning GEN-D12's `fn sample(&self, ctx: &EvalContext) -> f64` visitor, the noise/spline math primitives, the surface-rule interpreter, the feature-placement RNG-driven walk, and the jigsaw assembly algorithm) consumes these types read-only. Nowhere in this blueprint's deliverables does any function *evaluate* a density function, sample noise, or place a block.

### What JSON is consumed, and from where — resolving GEN-D7 precisely

GEN-D7's own text: `xtask fetch-worldgen-data <version>` "unzips the vanilla datapack embedded in the same legally-obtained `server.jar` NET-D9 already downloads (`data/minecraft/worldgen/**`, `data/minecraft/dimension/**`, `data/minecraft/dimension_type/**`...)." This is correct for nine of the ten JSON families this blueprint's schema covers — they are literal jar-resident resource files, extractable by a plain zip read with no Java invocation:

| Family | Jar-internal path | Files (26.2, per `docs/research/mc-26.2/05-worldgen.md §7` / `06-structures.md §7`) |
|---|---|---|
| Density functions | `data/minecraft/worldgen/density_function/**` | 35 (`overworld/`, `overworld_large_biomes/`, `overworld_amplified/`, `nether/`, `end/`, plus `shift_x.json`/`shift_y.json`/`shift_z.json`/`zero.json`) |
| Noise parameters | `data/minecraft/worldgen/noise/*.json` | 61 |
| Noise generator settings | `data/minecraft/worldgen/noise_settings/*.json` | 7 (`overworld`, `large_biomes`, `amplified`, `nether`, `end`, `caves`, `floating_islands`) |
| Configured carvers | `data/minecraft/worldgen/configured_carver/*.json` | 4 (`cave`, `cave_extra_underground`, `nether_cave`, `canyon`) |
| Configured features | `data/minecraft/worldgen/configured_feature/*.json` | 226 |
| Placed features | `data/minecraft/worldgen/placed_feature/*.json` | 262 |
| Structure sets | `data/minecraft/worldgen/structure_set/*.json` | 20 |
| Structures | `data/minecraft/worldgen/structure/*.json` | 31 |
| Template pools | `data/minecraft/worldgen/template_pool/**/*.json` | 188 |
| Processor lists | `data/minecraft/worldgen/processor_list/*.json` | — |

`fetch-worldgen-data`'s jar-unzip also extracts `data/minecraft/dimension/**` and `data/minecraft/dimension_type/**` (GEN-D7's own literal path list, restated verbatim in `extract.rs::run`'s Deliverables) into `worldgen_json_dir` alongside the ten families above, but **this blueprint defines no schema for them and `compile()` never reads them** — the task this blueprint was assigned names exactly the ten JSON node families in the table above (density function, noise, noise settings, biome parameter list, surface rules, configured/placed feature, configured carver, structure/structure-set/template-pool, processor list); dimension/dimension-type wiring (which `noise_generator_settings` preset each of overworld/nether/end actually uses) is left for whichever later blueprint owns the `GenStage` execution model (GEN-D25) to parse from this same already-extracted directory, with no re-extraction needed.

**One family is not jar-resident and needs a correction to GEN-D7's literal text**: the multi-noise biome **parameter lists**. `data/minecraft/worldgen/multi_noise_biome_source_parameter_list/{overworld,nether}.json` inside the jar contain only `{"preset": "minecraft:overworld"}` — the real 7594-entry overworld point list (and the 5-entry nether one) is Java-code-defined (`OverworldBiomeBuilder`) and only exists as JSON in the **data generator's report output**, `reports/biome_parameters/minecraft/{overworld,nether}.json` (confirmed, `docs/research/mc-26.2/05-worldgen.md §7`). This blueprint's `fetch-worldgen-data` therefore reuses **both** of M0-B08/M0-B07's shared primitives, not just the jar-unzip: `fetch_data::fetch_server_jar` (jar acquisition, shared with M0-B07) for the direct-unzip families above, **and** `fetch_data::run_data_reports` (the same `--reports` invocation M0-B07 already runs for `registries.json`/`blocks.json`) to additionally obtain `reports/biome_parameters/minecraft/*.json` — the same cached `datagen-output/<version>/generated/reports/` directory M0-B07's `codegen` verb already reads from, read a second time by this blueprint's own verb for a different pair of files inside it. No second `--reports` invocation, no second Java process — `run_data_reports` is idempotent (M0-B08's own contract) and this blueprint calls it exactly as M0-B07's `fetch.rs` does.

### Compiled representation and its committed home (GEN-D9, WS-D13-pattern)

GEN-D9: parse with `serde_json` (xtask/dev-time only, never in the release binary), canonicalize into `serde`-deriving Rust structs, encode with `postcard` into `crates/worldgen/generated/<protocol-version>/data.postcard`, `include_bytes!` + lazily deserialize once behind a `std::sync::OnceLock`. `12-workspace-structure.md`'s WS-D13 fixes `crates/registries/generated/<protocol-version>/` as registry-ID-table's home inside `rc-registries` — a **different** crate and a **different** kind of data (protocol-assigned integer IDs). This blueprint's output is GEN-D9's own, separately-named home, `crates/worldgen/generated/v776/`, inside `rc-worldgen` — a sibling application of the same "generated output lives beside its consuming crate, versioned by protocol version" convention WS-D13 establishes, not a reuse of `rc-registries`' directory. `rc-worldgen` already depends on `rc-registries` (per `12`'s dependency graph, unchanged by this blueprint) for whichever later blueprint needs to resolve a block-state/biome *name* to its registry-assigned numeric id; this blueprint's own compiled types store block/biome references as parsed-but-unresolved `ResourceLocation`/`BlockStateSpec` values (below), deliberately deferring that resolution to the evaluation blueprint that already has both this blueprint's graph and `rc-registries`' tables in scope.

### Commit-vs-regenerate boundary (GEN-D23/GEN-D24, ASSET-D16/ASSET-D25) — applied to this blueprint's exact artifacts

| Artifact | Committed? | Why |
|---|---|---|
| `server.jar` | Never. Cached under M0-B08's `.gitignore`d `oracle/<version>/server.jar`, reused as-is. | ASSET-D13/GEN-D23. |
| Raw extracted `datagen-output/<version>/worldgen-json/data/minecraft/worldgen/**` (this blueprint's own extraction output) and the reused `datagen-output/<version>/generated/reports/biome_parameters/**` | Never. Both land under the already-`.gitignore`d `/datagen-output/` tree (M0-B08). | GEN-D24: functional data is committable only in **compiled** form, never as raw extracted JSON — the exact rule NET-D10/M0-B07 already apply to `registries.json`/`blocks.json`. |
| `crates/worldgen/generated/v776/data.postcard` | Yes. | GEN-D24: worldgen JSON (excluding structure NBT templates) is functional/structural data, confirmed binding by ASSET-D25 — the same category NET-D10/ASSET-D15 already clear for registry tables. |
| `crates/worldgen/generated/v776/MANIFEST.json` | Yes. | Derived, non-creative metadata describing the already-committed blob (TEST-D47), same category as M0-B07's own manifest. |
| Structure NBT templates (`data/minecraft/structure/**.nbt`) | **Never extracted by this blueprint at all.** | GEN-D23/ASSET-D16: creative content, read at runtime by a later blueprint from an operator-supplied path — out of this blueprint's scope entirely; this blueprint's `fetch-worldgen-data` does not touch that jar path. |

### Field-name confidence and the reconciliation rule

Every JSON field name below is sourced either (a) directly from `docs/research/mc-26.2/05-worldgen.md`/`06-structures.md`/`17-noise-math.md`'s own quoted field names (marked **confirmed** inline, e.g. `old_blended_noise`'s five parameters, `NoiseGeneratorSettings`'s top-level field list, the four `RuleSource`/eleven `ConditionSource`/fifteen `PlacementModifierType` JSON type strings, `StructurePlacement`'s salt/spacing/separation fields), or (b) the well-known, independently-documented Mojang worldgen JSON schema (`minecraft.wiki`'s datapack pages, `misode`'s public worldgen visualizer — both ASSET-D18(b)/(e) allowed sources, and GEN-D8's own rationale already cites `misode/deepslate` as independent validation of this exact architecture) where the research corpus described behavior but not the literal field key (marked **moderate-confidence** inline). Every moderate-confidence field name is reconciled the first time `fetch-worldgen-data`/`compile-worldgen-data` actually run against a real jar (Implementation step 13); until then, this blueprint's binding **unknown-field policy** (Constraints (d)) makes any name mismatch a loud, actionable deserialize error rather than a silent data loss, so reconciliation is self-checking, not a leap of faith.

### Density-function node enumeration — re-verified against 26.2, correcting GEN-D12

GEN-D12 lists 30 node kinds and flags itself as needing re-verification ("re-verified — not assumed unchanged — on every NET-D2 version bump"). Cross-checked against `docs/research/mc-26.2/05-worldgen.md §3.3`'s own table (itself checked against the ASSET-D18(f) reference, confirmed, per that document, current for 26.2), the real enumeration has **34 JSON `type` discriminators**, and GEN-D12's list is missing four: `invert`, `squeeze` (both `Mapped.Type` unary transforms), `shift` (the legacy 3-D shift field, distinct from `shift_a`/`shift_b`), and `find_top_surface` (new in 26.2 — used to build `preliminarySurfaceLevel`). This blueprint's `DensityFunctionJson` enum (Deliverables) uses the corrected, complete 34-variant list; `find_top_surface`'s exact field shape is not stated anywhere in the research corpus beyond its algorithm description, so it is modeled as a zero-field marker (moderate-confidence, flagged for reconciliation) — the same treatment already given to `beardifier`/`end_islands`, which are also runtime-rewired rather than JSON-parameterized. `MulOrAdd` (a runtime-only decode optimization the research doc explicitly calls out as "not a distinct wire format... a from-scratch density-function codec only needs to implement `add`/`mul`/`min`/`max`") is correctly **not** a 35th variant.

### Reference resolution

Two distinct reference mechanisms appear across these JSON families, both resolved by this blueprint's `compile()`:

1. **Density-function/noise named references** (vanilla's `HolderHolder` indirection): a density-function field may be a bare number (→ `Constant`), an inline object (→ nested node), or a `"namespace:path"` string naming another `density_function`/`noise` registry entry. `compile()` resolves every string reference to a `DensityFunctionId`/`NoiseParamId` by looking it up in the interned name table built from every file in the corresponding family (Interning below); an unresolvable name is `CompileError::DanglingReference`.
2. **Tag references**: any `HolderSet`-typed field (structure `biomes`, placement-modifier `biome` checks) may be a single string starting with `#` (a tag, e.g. `"#minecraft:is_forest"`) instead of an inline list — this blueprint stores tag references **verbatim** as an opaque `BiomeSet::Tag(String)` (the `#`-prefixed name), performing no tag-membership expansion (that requires the `data/minecraft/tags/**` tree, out of this blueprint's ten named JSON families and out of scope — a later blueprint's job once tag data has its own extraction pass).

### Interning and determinism (mirrors M0-B07's own rules exactly, restated for this blueprint's data shape)

`compile()` must be a pure function: identical logical JSON content (regardless of filesystem walk order) produces byte-identical `WorldgenData`/`postcard` output. Binding rules: (1) every raw-JSON map is parsed into `BTreeMap<ResourceLocation, _>`, never `HashMap`; (2) every compiled table interns its entries in **ascending `ResourceLocation` string order** (`namespace:path`, byte-wise) — this is the sort key for assigning `DensityFunctionId(0)`, `DensityFunctionId(1)`, … and every other interned id type, an explicit, stable rule independent of any `HashMap`/filesystem iteration order; (3) no wall-clock timestamp, hostname, or run-varying value appears anywhere in `WorldgenData` or its `MANIFEST.json`; (4) `serde_json::Value`'s default (non-`preserve_order`) `Map` backing is itself a `BTreeMap`, confirmed by this workspace's `serde_json = "1.0.151"` pin carrying no `preserve_order` feature (`12-workspace-structure.md`'s Workspace Dependency Versions table) — every opaque `serde_json::Value` payload this blueprint stores (feature configs, non-jigsaw structure extras) is therefore already deterministically ordered with zero extra work.

### Why `serde_json::Value` is the opaque-payload type, and where it is used

Three of the ten JSON families carry deeply nested, wide (63-feature-type / 40-config-record / 15-piece-type) per-entry shapes whose full structural enumeration is explicitly this blueprint's *evaluation*-adjacent neighbor's job, not this blueprint's (Scope boundary above): a `configured_feature`'s `config` object (feature-specific parameters — tree shapes, ore-vein configs, …), a non-`jigsaw` `structure`'s type-specific extra fields (`mineshaft_type`, `biome_temp`, `setups`, …), and any placement-modifier field this blueprint does not otherwise name explicitly. Rather than inventing a bespoke recursive value type, this blueprint reuses `serde_json::Value` directly — it already implements `serde::Serialize`/`Deserialize` generically (works with `postcard`'s serializer/deserializer exactly as with `serde_json`'s own, since both are ordinary `serde` backends) and its default `Map` backing is deterministic (Interning above). `rc-worldgen`'s `Cargo.toml` gains a `serde_json` dependency for this reason alone — `WorldgenData` itself contains `serde_json::Value` fields. A later feature-placement blueprint is expected to replace these opaque payloads with fully-typed Rust structs per feature/config kind once it needs to *evaluate* them; until then, generic-but-validated (every payload still round-trips through `serde_json`'s own parser, so malformed JSON is still caught) is the right level of investment for "parsing/validation/graph shape."

### Error taxonomy (fail-fast, actionable paths — restated as this blueprint's binding policy)

`compile()` collects **every** error rather than stopping at the first (friendlier for a human fixing many mismatches after a version bump at once), but the overall gate is still fail-fast in the sense GEN-D1 requires: **any** non-empty error list blocks `compile-worldgen-data` from writing `data.postcard`/`MANIFEST.json` at all — there is no partial-success output. Every error variant (Deliverables' `CompileError`) carries the offending file's `ResourceLocation` (and, where applicable, the specific field name) so a human can jump straight to the JSON that needs fixing, never a bare "compilation failed."

## Deliverables

### `xtask/Cargo.toml` (modify — add one dependency)

```toml
[dependencies]
# ...existing lines (clap, xshell, serde, serde_json, sha1, sha2) unchanged...
zip = { workspace = true }              # M5-B02: fetch-worldgen-data's server.jar unzip step (already pinned, rc-assets, 12-workspace-structure.md)
rc-worldgen = { path = "../crates/worldgen" }  # M5-B02: compile.rs constructs rc_worldgen::data::WorldgenData directly (Deliverables) — xtask is dev-only tooling, not part of WS-D3's SIM/NETRENDER sets, so this edge is unconstrained by lint-deps
```

### `crates/worldgen/Cargo.toml` (modify — add two dependencies)

```toml
[dependencies]
rc-core = { path = "../core" }
rc-chunk-storage = { path = "../chunk-storage" }
rc-registries = { path = "../registries" }
serde = { workspace = true }        # M5-B02: WorldgenData's derive(Serialize, Deserialize)
serde_json = { workspace = true }   # M5-B02: opaque Value payloads, Context
postcard = { workspace = true }     # M5-B02: data.postcard decode, GEN-D9
```

(`rc-core`/`rc-chunk-storage`/`rc-registries` are M0-B01's existing lines, reproduced for a complete file. `xtask lint-deps` is unaffected: `rc-worldgen` is not in `SIM`/`NETRENDER`, and none of these three additions creates an edge into either set.)

### `xtask/src/worldgen_data/schema/common.rs`

```rust
//! Shared primitives every worldgen JSON family's schema reuses.

use std::collections::BTreeMap;

/// `namespace:path`, defaulting namespace to `"minecraft"` when the input carries no
/// `:` (vanilla's own shorthand — every worldgen JSON reference may omit the namespace).
/// Serializes/deserializes as a single string (`namespace:path`), not two fields, so its
/// compiled-side twin (`rc-worldgen`'s own `ResourceLocation`, identical shape) round-
/// trips through `postcard` as compactly as a plain `String`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceLocation {
    pub namespace: String,
    pub path: String,
}

impl ResourceLocation {
    /// `"namespace:path"` -> `Self`; `"path"` (no `:`) -> `namespace = "minecraft"`.
    /// `Err` on an empty path or more than one `:`.
    pub fn parse(s: &str) -> Result<Self, String>;
    pub fn as_string(&self) -> String;
}
// impl serde::Serialize/Deserialize via #[serde(try_from = "String", into = "String")]
// backed by parse/as_string, so every `ResourceLocation` field below deserializes
// directly from a JSON string with no wrapper object.

/// A `HolderSet`-typed reference: either a single `"#namespace:path"` tag (stored
/// verbatim, `#` included) or an inline list. Tag membership is never expanded by this
/// blueprint (Context's "Reference resolution").
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum TagOrList<T> {
    Tag(String),
    List(Vec<T>),
}

/// A parsed-but-unresolved block-state reference (`"minecraft:stone"` or
/// `"minecraft:oak_door[facing=east,half=lower]"`), used by surface-rule `block` leaves,
/// structure-processor `RuleTest`/output states, and carver replaceable-block sets.
/// Numeric `BlockStateId` resolution against `rc_registries::generated_v776` is a later
/// blueprint's job (Context).
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(try_from = "String")]
pub struct BlockStateSpec {
    pub block: ResourceLocation,
    pub properties: BTreeMap<String, String>,
}
impl TryFrom<String> for BlockStateSpec {
    type Error = String;
    /// Splits on the first `[`, block id before it, `key=value` comma list (trailing
    /// `]` stripped) after it; no `[` at all -> empty `properties`.
    fn try_from(s: String) -> Result<Self, String>;
}

/// 6 `HeightProvider` kinds (`docs/research/mc-26.2/05-worldgen.md §2/§4` package table
/// row — JSON type strings are the standard-convention snake_case of each Java class
/// name, moderate-confidence, reconciled per Context). Used by carver Y distribution and
/// the `height_range` placement modifier.
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum HeightProviderJson {
    Constant { value: VerticalAnchorJson },
    Uniform { min_inclusive: VerticalAnchorJson, max_inclusive: VerticalAnchorJson },
    BiasedToBottom { min_inclusive: VerticalAnchorJson, max_inclusive: VerticalAnchorJson, #[serde(default)] inner: Option<i32> },
    VeryBiasedToBottom { min_inclusive: VerticalAnchorJson, max_inclusive: VerticalAnchorJson, #[serde(default)] inner: Option<i32> },
    Trapezoid { min_inclusive: VerticalAnchorJson, max_inclusive: VerticalAnchorJson, #[serde(default)] plateau: Option<i32> },
    WeightedList { distribution: Vec<WeightedHeightEntryJson> },
    /// A bare `VerticalAnchorJson` where a `HeightProvider` is expected decodes as
    /// `Constant` (vanilla's own bare-value shorthand, same pattern as
    /// `DensityFunctionRef::Number`). Handled by a custom `Deserialize` that first tries
    /// the tagged form above, falling back to this.
    #[serde(skip)]
    ConstantShorthand(VerticalAnchorJson),
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct WeightedHeightEntryJson { pub data: Box<HeightProviderJson>, pub weight: u32 }

/// `{"absolute": Y}` | `{"above_bottom": N}` | `{"below_top": N}` (moderate-confidence
/// shape, `05-worldgen.md`'s `VerticalAnchor` class reference; reconciled per Context).
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum VerticalAnchorJson {
    Absolute { absolute: i32 },
    AboveBottom { above_bottom: i32 },
    BelowTop { below_top: i32 },
}

/// A bare int decodes as `Constant`; anything more complex falls back to a raw,
/// unvalidated `serde_json::Value` (Context's opaque-payload rule) — this blueprint does
/// not enumerate every `IntProvider` kind, since none of this family's own graph-shape
/// fields need more than the common constant/uniform cases to resolve correctly.
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum IntProviderJson {
    Constant(i32),
    Uniform { #[serde(rename = "type")] _t: UniformIntTag, min_inclusive: i32, max_inclusive: i32 },
    Other(serde_json::Value),
}
#[derive(serde::Deserialize, Debug, Clone)]
pub struct UniformIntTag; // matches literal "minecraft:uniform" via a custom Deserialize
```

### `xtask/src/worldgen_data/schema/density_function.rs`

The complete 34-variant enumeration (Context). `DensityFunctionRef` is the recursive child-slot type every binary/unary node uses; a bare JSON number decodes as `Constant`, a bare string as a named reference, an object as a nested node — vanilla's own three-shape codec, reproduced exactly.

```rust
use super::common::ResourceLocation;

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum DensityFunctionRef {
    Number(f64),
    Reference(String),
    Inline(Box<DensityFunctionJson>),
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum DensityFunctionJson {
    // --- arithmetic (confirmed field names: `misode` public worldgen schema, moderate-confidence) ---
    Constant { argument: f64 },
    Add { argument1: DensityFunctionRef, argument2: DensityFunctionRef },
    Mul { argument1: DensityFunctionRef, argument2: DensityFunctionRef },
    Min { argument1: DensityFunctionRef, argument2: DensityFunctionRef },
    Max { argument1: DensityFunctionRef, argument2: DensityFunctionRef },
    Abs { argument: DensityFunctionRef },
    Square { argument: DensityFunctionRef },
    Cube { argument: DensityFunctionRef },
    HalfNegative { argument: DensityFunctionRef },
    QuarterNegative { argument: DensityFunctionRef },
    Invert { argument: DensityFunctionRef },
    Squeeze { argument: DensityFunctionRef },
    Clamp { input: DensityFunctionRef, min: f64, max: f64 },
    // --- noise sampling ---
    Noise { noise: String, xz_scale: f64, y_scale: f64 },
    ShiftedNoise { noise: String, xz_scale: f64, y_scale: f64, shift_x: DensityFunctionRef, shift_y: DensityFunctionRef, shift_z: DensityFunctionRef },
    ShiftA { argument: String },
    ShiftB { argument: String },
    Shift { argument: String },
    OldBlendedNoise { xz_scale: f64, y_scale: f64, xz_factor: f64, y_factor: f64, smear_scale_multiplier: f64 }, // confirmed field names, 05-worldgen.md §3.6
    // --- interval / mapping ---
    RangeChoice { input: DensityFunctionRef, min_inclusive: f64, max_exclusive: f64, when_in_range: DensityFunctionRef, when_out_of_range: DensityFunctionRef },
    /// `thresholds.len() + 1 == branches.len()` (GEN-D12's "N+1 functions" — validated
    /// by `compile()`, Implementation steps). Field names moderate-confidence.
    IntervalSelect { input: DensityFunctionRef, thresholds: Vec<f64>, branches: Vec<DensityFunctionRef> },
    Spline { spline: SplineJson },
    YClampedGradient { from_y: i32, to_y: i32, from_value: f64, to_value: f64 },
    FindTopSurface {}, // moderate-confidence zero-field marker, Context
    // --- caching / marker nodes (single-child wrappers) ---
    Interpolated { argument: DensityFunctionRef },
    FlatCache { argument: DensityFunctionRef },
    /// `#[serde(rename = "cache_2d")]` — the blanket `rename_all = "snake_case"` on this
    /// enum does not insert an underscore before a trailing digit (`Cache2d` would
    /// otherwise decode as `"cache2d"`, not the real `"cache_2d"` discriminator), so
    /// this one variant needs an explicit override.
    #[serde(rename = "cache_2d")]
    Cache2d { argument: DensityFunctionRef },
    CacheOnce { argument: DensityFunctionRef },
    CacheAllInCell { argument: DensityFunctionRef },
    // --- world-integration marker nodes (zero-field; runtime-rewired, GEN-D12) ---
    BlendDensity { argument: DensityFunctionRef },
    BlendAlpha {},
    BlendOffset {},
    Beardifier {},
    EndIslands {},
}

/// Mirrors `CubicSpline<C>` exactly (`docs/research/mc-26.2/17-noise-math.md §3.7`,
/// confirmed field shape): a spline is either a bare constant or a strictly-ascending
/// `locations` list, each with its own (possibly nested) `values` spline and `derivatives`.
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum SplineJson {
    Constant(f32),
    Multipoint { coordinate: DensityFunctionRef, points: Vec<SplinePointJson> },
}
#[derive(serde::Deserialize, Debug, Clone)]
pub struct SplinePointJson { pub location: f32, pub value: SplineJson, pub derivative: f32 }
```

### `xtask/src/worldgen_data/schema/noise_settings.rs`

```rust
use super::common::{HeightProviderJson, ResourceLocation};
use super::density_function::DensityFunctionJson;
use std::collections::BTreeMap;

/// `worldgen/noise/*.json` — `NormalNoise.NoiseParameters` (61 files).
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct NoiseParamsJson { pub first_octave: i32, pub amplitudes: Vec<f64> }

/// `NoiseSettings`'s four ints (`05-worldgen.md §3.5`; field names moderate-confidence,
/// standard public datapack schema).
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct NoiseDimensionsJson {
    pub min_y: i32,
    pub height: i32,
    pub size_horizontal: i32,
    pub size_vertical: i32,
}

/// The fixed 15-slot `NoiseRouter` (GEN-D13, confirmed exact 15-name list:
/// `final_density`, `preliminary_surface_level`, `barrier`/`fluid_level_floodedness`/
/// `fluid_level_spread`/`lava` (feed `Aquifer`), `temperature`/`vegetation`/
/// `continents`/`erosion`/`depth`/`ridges` (feed `Climate.Sampler`), `vein_toggle`/
/// `vein_ridged`/`vein_gap` — cross-checked against `05-worldgen.md §3.4`'s Java field
/// names, `barrierNoise`/`fluidLevelFloodednessNoise`/`fluidLevelSpreadNoise`/
/// `lavaNoise` carrying an explicit `Noise` suffix Java-side that the other eleven
/// fields do not — reproduced below as `barrier_noise`/`fluid_level_floodedness_noise`/
/// `fluid_level_spread_noise`/`lava_noise` accordingly). The *set of 15 slots* is
/// confirmed; exact JSON casing is moderate-confidence, reconciled per Context.
///
/// Every slot is a `DensityFunctionRef`, **not** a bare `DensityFunctionJson`: real
/// `noise_settings` files overwhelmingly wire each slot to a *named* entry under
/// `data/minecraft/worldgen/density_function/**` (`NoiseRouterData`'s own "named
/// intermediate `DensityFunction` registry entries... so multiple presets can reuse the
/// same field," `05-worldgen.md §3.4`) rather than inlining the whole graph per preset —
/// this is exactly the string-reference shape `DensityFunctionRef::Reference` exists to
/// decode.
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct NoiseRouterJson {
    pub barrier_noise: DensityFunctionRef,
    pub fluid_level_floodedness_noise: DensityFunctionRef,
    pub fluid_level_spread_noise: DensityFunctionRef,
    pub lava_noise: DensityFunctionRef,
    pub temperature: DensityFunctionRef,
    pub vegetation: DensityFunctionRef,
    pub continents: DensityFunctionRef,
    pub erosion: DensityFunctionRef,
    pub depth: DensityFunctionRef,
    pub ridges: DensityFunctionRef,
    pub preliminary_surface_level: DensityFunctionRef,
    pub final_density: DensityFunctionRef,
    pub vein_toggle: DensityFunctionRef,
    pub vein_ridged: DensityFunctionRef,
    pub vein_gap: DensityFunctionRef,
}

/// `SurfaceRules.RuleSource`, 4 types (confirmed strings, `05-worldgen.md §3.9`).
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum SurfaceRuleJson {
    Sequence { sequence: Vec<SurfaceRuleJson> },
    Condition { if_true: SurfaceConditionJson, then_run: Box<SurfaceRuleJson> },
    Block { result_state: super::common::BlockStateSpec },
    Bandlands {},
}

/// `SurfaceRules.ConditionSource`, 11 types (confirmed strings, `05-worldgen.md §3.9`).
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum SurfaceConditionJson {
    Biome { biome_is: Vec<ResourceLocation> },
    NoiseThreshold { noise: String, min_threshold: f64, max_threshold: f64 },
    VerticalGradient { random_name: String, true_at_and_below: super::common::VerticalAnchorJson, false_at_and_above: super::common::VerticalAnchorJson },
    YAbove { anchor: super::common::VerticalAnchorJson, surface_depth_multiplier: i32, add_stone_depth: bool },
    Water { offset: i32, surface_depth_multiplier: i32, add_stone_depth: bool },
    Temperature {},
    Steep {},
    Hole {},
    AbovePreliminarySurface {},
    Not { invert: Box<SurfaceConditionJson> },
    StoneDepth { offset: i32, add_surface_depth: bool, secondary_depth_range: i32, surface_type: StoneDepthSurfaceTypeJson },
}
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StoneDepthSurfaceTypeJson { Floor, Ceiling }

/// One `spawn_target` entry — a 6-axis (7th "offset" axis omitted, always 0 for a spawn
/// target per vanilla) climate point, reusing the biome-parameter-list point shape.
pub type SpawnTargetPointJson = super::biome::BiomeParameterPointJson;

/// `worldgen/noise_settings/*.json` — the full per-preset bundle (confirmed exact
/// top-level field list, `05-worldgen.md §4`'s `NoiseGeneratorSettings` row).
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct NoiseGeneratorSettingsJson {
    pub noise: NoiseDimensionsJson,
    pub default_block: super::common::BlockStateSpec,
    pub default_fluid: super::common::BlockStateSpec,
    pub noise_router: NoiseRouterJson,
    pub surface_rule: SurfaceRuleJson,
    pub spawn_target: Vec<SpawnTargetPointJson>,
    pub sea_level: i32,
    pub disable_mob_generation: bool,
    pub aquifers_enabled: bool,
    pub ore_veins_enabled: bool,
    pub legacy_random_source: bool,
}
```

### `xtask/src/worldgen_data/schema/biome.rs`

```rust
/// One entry of `reports/biome_parameters/minecraft/{overworld,nether}.json`
/// (confirmed shape, `05-worldgen.md §7`): `parameters` maps each of the 7 climate
/// axes to either a scalar point or a `[min, max]` span.
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct BiomeParameterEntryJson {
    pub biome: super::common::ResourceLocation,
    pub parameters: BiomeParameterPointJson,
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct BiomeParameterPointJson {
    pub temperature: ClimateSpanJson,
    pub humidity: ClimateSpanJson,
    pub continentalness: ClimateSpanJson,
    pub erosion: ClimateSpanJson,
    pub depth: ClimateSpanJson,
    pub weirdness: ClimateSpanJson,
    #[serde(default)]
    pub offset: Option<f32>,
}

/// A scalar point (`min == max`) or a `[min, max]` span, both in raw `f32` units
/// (quantized only at compile time — Deliverables' `quantize_climate`).
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ClimateSpanJson {
    Point(f32),
    Span([f32; 2]),
}
```

### `xtask/src/worldgen_data/schema/feature.rs`

```rust
use super::common::{HeightProviderJson, IntProviderJson, ResourceLocation, TagOrList};

/// `worldgen/configured_carver/*.json` (4 files). Fields per `docs/research/mc-26.2/
/// 05-worldgen.md §3.12`'s described shape; `probability`/`y`/replaceable-block-set
/// confirmed present, exact key names moderate-confidence.
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ConfiguredCarverJson {
    #[serde(rename = "type")]
    pub carver_type: String, // "minecraft:cave" | "minecraft:canyon" (2 WorldCarver types, 4 configured instances)
    pub probability: f32,
    pub y: HeightProviderJson,
    #[serde(default)]
    pub y_scale: Option<serde_json::Value>, // canyon-only field, shape not confirmed — opaque, Context
    pub lava_level: super::common::VerticalAnchorJson,
    pub replaceable: TagOrList<ResourceLocation>,
    #[serde(default)]
    pub debug_settings: Option<serde_json::Value>,
}

/// `worldgen/configured_feature/*.json` (226 files). `config`'s inner shape is
/// intentionally opaque (Context's "why serde_json::Value").
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ConfiguredFeatureJson {
    #[serde(rename = "type")]
    pub feature_type: ResourceLocation,
    pub config: serde_json::Value,
}

/// `worldgen/placed_feature/*.json` (262 files). `feature` names a `configured_feature`
/// by id.
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct PlacedFeatureJson {
    pub feature: String,
    pub placement: Vec<PlacementModifierJson>,
}

/// All 15 `PlacementModifierType`s (confirmed strings, `05-worldgen.md §3.13`).
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlacementModifierJson {
    BlockPredicateFilter { predicate: serde_json::Value },
    RarityFilter { chance: u32 },
    SurfaceRelativeThresholdFilter { heightmap: String, #[serde(default)] min_inclusive: Option<i32>, #[serde(default)] max_inclusive: Option<i32> },
    SurfaceWaterDepthFilter { max_water_depth: i32 },
    Biome {},
    Count { count: IntProviderJson },
    NoiseBasedCount { noise_to_count_ratio: f64, noise_factor: f64, #[serde(default)] noise_offset: f64 },
    NoiseThresholdCount { noise_level: f64, below_noise: i32, above_noise: i32 },
    CountOnEveryLayer { count: IntProviderJson },
    EnvironmentScan { direction_of_search: String, #[serde(default)] target_condition: Option<serde_json::Value>, #[serde(default)] allowed_search_condition: Option<serde_json::Value>, max_steps: i32 },
    Heightmap { heightmap: String },
    HeightRange { height: HeightProviderJson },
    InSquare {},
    RandomOffset { xz_spread: IntProviderJson, y_spread: IntProviderJson },
    FixedPlacement { positions: Vec<[i32; 3]> },
}
```

### `xtask/src/worldgen_data/schema/structure.rs`

```rust
use super::common::{BlockStateSpec, HeightProviderJson, ResourceLocation, TagOrList};
use std::collections::BTreeMap;

/// The 11 `GenerationStep.Decoration` values (confirmed exact list and order,
/// `05-worldgen.md §4`).
#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecorationStep {
    RawGeneration, Lakes, LocalModifications, UndergroundStructures, SurfaceStructures,
    Strongholds, UndergroundOres, UndergroundDecoration, FluidSprings, VegetalDecoration,
    TopLayerModification,
}

/// `TerrainAdjustment`'s 5 values (confirmed enum members, `06-structures.md §3.1`).
#[derive(serde::Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TerrainAdaptation { #[default] None, Bury, BeardThin, BeardBox, Encapsulate }

/// `worldgen/structure/*.json` (31 files). Common `StructureSettings` fields
/// (confirmed, `06-structures.md §3.1`) are typed; every type-specific field (jigsaw's
/// `start_pool`/`size`/… included) lands in `extra` (Context's opaque-payload rule) —
/// this blueprint validates the common shape and every family's *cross-references*
/// (biomes tag, template-pool ids inside `extra` are NOT reference-checked, since they
/// live inside an opaque value; jigsaw-specific reference resolution against
/// `template_pool`/`processor_list` is a later blueprint's evaluation-layer job).
#[derive(serde::Deserialize, Debug, Clone)]
pub struct StructureJson {
    #[serde(rename = "type")]
    pub structure_type: String, // one of 15 StructureType ids, confirmed list §3.1
    pub biomes: TagOrList<ResourceLocation>,
    pub step: DecorationStep,
    #[serde(default)]
    pub terrain_adaptation: TerrainAdaptation,
    #[serde(default)]
    pub spawn_overrides: BTreeMap<String, serde_json::Value>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// `worldgen/structure_set/*.json` (20 files, confirmed shape §3.2/§7).
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct StructureSetJson {
    pub placement: StructurePlacementJson,
    pub structures: Vec<StructureSelectionEntryJson>,
}
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct StructureSelectionEntryJson { pub structure: String, pub weight: u32 }

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum StructurePlacementJson {
    RandomSpread {
        salt: i64,
        spacing: u32,
        separation: u32,
        #[serde(default)]
        spread_type: RandomSpreadTypeJson,
        #[serde(default)]
        frequency_reduction_method: Option<String>,
        #[serde(default)]
        frequency: Option<f32>,
        #[serde(default)]
        locate_offset: Option<[i32; 3]>,
        #[serde(default)]
        exclusion_zone: Option<serde_json::Value>,
    },
    ConcentricRings {
        salt: i64,
        distance: u32,
        spread: u32,
        count: u32,
        preferred_biomes: TagOrList<ResourceLocation>,
        #[serde(default)]
        locate_offset: Option<[i32; 3]>,
    },
}
#[derive(serde::Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub enum RandomSpreadTypeJson { #[default] Linear, Triangular }

/// `worldgen/template_pool/**/*.json` (188 files, confirmed shape §7).
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct TemplatePoolJson {
    pub fallback: String,
    pub elements: Vec<WeightedPoolElementJson>,
}
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct WeightedPoolElementJson { pub element: PoolElementJson, pub weight: u32 }

/// The 5 `StructurePoolElementType`s (confirmed list, `06-structures.md §3.5`).
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "element_type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PoolElementJson {
    SinglePoolElement { location: String, #[serde(default)] processors: Option<String>, #[serde(default)] projection: ProjectionJson },
    LegacySinglePoolElement { location: String, #[serde(default)] processors: Option<String>, #[serde(default)] projection: ProjectionJson },
    FeaturePoolElement { feature: String, #[serde(default)] projection: ProjectionJson },
    ListPoolElement { elements: Vec<PoolElementJson>, #[serde(default)] projection: ProjectionJson },
    EmptyPoolElement {},
}
#[derive(serde::Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionJson { #[default] Rigid, TerrainMatching }

/// `worldgen/processor_list/*.json` (confirmed field/shape, §3.8/§3.9/§7).
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ProcessorListJson { pub processors: Vec<StructureProcessorJson> }

/// All 11 `StructureProcessor` kinds (confirmed registry ids, §3.8).
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "processor_type", rename_all = "snake_case", deny_unknown_fields)]
pub enum StructureProcessorJson {
    BlackstoneReplace {},
    BlockAge { mossiness: f32 },
    BlockIgnore { blocks: Vec<ResourceLocation> },
    BlockRot { #[serde(default)] rottable_blocks: Option<ResourceLocation>, integrity: f32 },
    Capped { delegate: Box<StructureProcessorJson>, limit: super::common::IntProviderJson },
    Gravity { heightmap: String, offset: i32 },
    JigsawReplacement {},
    LavaSubmergedBlock {},
    Nop {},
    ProtectedBlocks { value: TagOrList<ResourceLocation> },
    Rule { rules: Vec<ProcessorRuleJson> },
}
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ProcessorRuleJson {
    pub input_predicate: RuleTestJson,
    pub location_predicate: RuleTestJson,
    #[serde(default)]
    pub position_predicate: Option<PosRuleTestJson>,
    pub output_state: BlockStateSpec,
    #[serde(default)]
    pub block_entity_modifier: Option<serde_json::Value>,
}
/// 6 `RuleTest` kinds (confirmed registry list, §3.8).
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "predicate_type", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuleTestJson {
    AlwaysTrue {},
    BlockMatch { block: ResourceLocation },
    BlockstateMatch { block_state: BlockStateSpec },
    TagMatch { tag: String },
    RandomBlockMatch { block: ResourceLocation, probability: f32 },
    RandomBlockstateMatch { block_state: BlockStateSpec, probability: f32 },
}
/// 3 `PosRuleTest` kinds (confirmed registry list, §3.8).
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(tag = "predicate_type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PosRuleTestJson {
    AlwaysTrue {},
    LinearPos { min_chance: f32, max_chance: f32, min_dist: i32, max_dist: i32 },
    AxisAlignedLinearPos { min_chance: f32, max_chance: f32, min_dist: i32, max_dist: i32, axis: String },
}
```

### `xtask/src/worldgen_data/error.rs`

```rust
use super::schema::common::ResourceLocation;

#[derive(Debug, Clone)]
pub enum CompileError {
    /// A `type`/`element_type`/`predicate_type` discriminator this blueprint's schema
    /// does not recognize — GEN-D9's closed enum, hit on encountering something outside
    /// it (a real version-bump signal, or an extraction bug; never silently ignored).
    UnknownNodeKind { file: ResourceLocation, kind: String },
    /// serde-level parse failure (includes serde's own "unknown field" rejections from
    /// every `deny_unknown_fields` struct above — Constraints (d)).
    ParseError { file: ResourceLocation, message: String },
    /// A named string reference (density function, noise, configured_feature, template
    /// pool, processor list, structure, structure set) that does not resolve to any
    /// interned entry.
    DanglingReference { from: ResourceLocation, field: String, target: String },
    /// A cycle among named density-function/noise references (Context).
    ReferenceCycle { cycle: Vec<ResourceLocation> },
    /// `IntervalSelect` where `thresholds.len() + 1 != branches.len()`.
    ArityMismatch { file: ResourceLocation, message: String },
    Io { path: std::path::PathBuf, message: String },
}
impl std::fmt::Display for CompileError { /* one line per variant, always naming the file/field */ }
```

### `xtask/src/worldgen_data/intern.rs`

```rust
use std::collections::BTreeMap;

/// Assigns ascending, stable ids (Context's determinism rule 2) to a
/// `BTreeMap<ResourceLocation, T>`'s entries, in `ResourceLocation` string order.
/// Generic over the compiled id newtype `Id` (e.g. `rc_worldgen::data::DensityFunctionId`)
/// via a caller-supplied `mk_id: impl Fn(u32) -> Id`.
pub struct Interner<Id> {
    pub ids_by_name: BTreeMap<String, Id>,
    pub next_index: u32,
}
impl<Id: Copy> Interner<Id> {
    pub fn new() -> Self;
    /// Interns `name` if not already present, returning its id either way.
    pub fn intern(&mut self, name: &str, mk_id: impl Fn(u32) -> Id) -> Id;
    pub fn get(&self, name: &str) -> Option<Id>;
}
```

### `xtask/src/worldgen_data/compile.rs`

```rust
use super::error::CompileError;
use super::schema::*; // xtask's own ResourceLocation (parse-side) among everything else
use rc_worldgen::data as compiled; // the compiled types Deliverables define, aliased —
                                    // `compiled::ResourceLocation` is a DIFFERENT type
                                    // from this file's own (schema-side) `ResourceLocation`
                                    // above; a glob-import of both would collide, so this
                                    // module always spells the compiled side out as
                                    // `compiled::WorldgenData`, `compiled::DensityFunctionNode`, etc.
use std::collections::BTreeMap;

/// Everything `read_raw_worldgen_json` (below) reads off disk, one `BTreeMap` per JSON
/// family, keyed by each file's own resource location.
pub struct RawWorldgenJson {
    pub density_functions: BTreeMap<ResourceLocation, DensityFunctionJson>,
    pub noise_params: BTreeMap<ResourceLocation, NoiseParamsJson>,
    pub noise_generator_settings: BTreeMap<ResourceLocation, NoiseGeneratorSettingsJson>,
    /// Keyed by dimension family (`"minecraft:overworld"`, `"minecraft:nether"`), value
    /// = that file's flat entry list (Context's biome_parameters correction).
    pub biome_parameter_lists: BTreeMap<ResourceLocation, Vec<BiomeParameterEntryJson>>,
    pub configured_carvers: BTreeMap<ResourceLocation, ConfiguredCarverJson>,
    pub configured_features: BTreeMap<ResourceLocation, ConfiguredFeatureJson>,
    pub placed_features: BTreeMap<ResourceLocation, PlacedFeatureJson>,
    pub structure_sets: BTreeMap<ResourceLocation, StructureSetJson>,
    pub structures: BTreeMap<ResourceLocation, StructureJson>,
    pub template_pools: BTreeMap<ResourceLocation, TemplatePoolJson>,
    pub processor_lists: BTreeMap<ResourceLocation, ProcessorListJson>,
}

/// Pure transform: raw parsed JSON in, the compiled, id-interned `WorldgenData` out (or
/// every validation error found — Context's "collect, don't stop at first"). No
/// filesystem access; deterministic per Context's Interning rules.
pub fn compile(raw: &RawWorldgenJson, protocol_version: u32, mc_version: &str) -> Result<compiled::WorldgenData, Vec<CompileError>>;
```

### `xtask/src/worldgen_data/extract.rs`

```rust
pub struct FetchWorldgenArgs {
    pub version: String,
    pub server_jar: Option<std::path::PathBuf>,
    pub offline: bool,
}

pub struct FetchWorldgenOutcome {
    /// `datagen-output/<version>/worldgen-json/` — root of the re-created jar-internal
    /// `data/minecraft/{worldgen,dimension,dimension_type}/**` tree (GEN-D7).
    pub worldgen_json_dir: std::path::PathBuf,
    /// `datagen-output/<version>/generated/reports/` — M0-B08's shared
    /// `run_data_reports` output dir, reused for `biome_parameters/**` (Context).
    pub reports_dir: std::path::PathBuf,
    pub jar_sha1: String,
}

/// Orchestrates `fetch-worldgen-data`: reuses `crate::fetch_data::fetch_server_jar`
/// (jar acquisition, identical to M0-B07's own `fetch.rs`) and
/// `crate::fetch_data::run_data_reports` (for `biome_parameters/**`, Context), then
/// unzips `data/minecraft/{worldgen,dimension,dimension_type}/**` from the jar itself
/// via the `zip` crate into `worldgen_json_dir`, preserving each entry's own internal
/// path. Never touches `data/minecraft/structure/**.nbt` (GEN-D23 — that jar path is
/// never read by this function). `--server-jar`/`--offline` semantics mirror M0-B07's
/// `fetch.rs::run` exactly (same two CLI-level cases, same shared-primitive reuse).
pub fn run(args: &FetchWorldgenArgs) -> Result<FetchWorldgenOutcome, String>;

/// Walks `worldgen_json_dir` + `reports_dir` and deserializes every file into
/// `compile::RawWorldgenJson`. Each file's `ResourceLocation` is derived from its path
/// relative to its own family directory (`.json` stripped, `/`-joined subdirectory
/// segments kept as the path's own `/`-separated tail — e.g.
/// `density_function/overworld/continents.json` under the `minecraft` namespace root ->
/// `minecraft:overworld/continents`). A file that fails to parse under its family's
/// schema produces `CompileError::ParseError`, collected (not returned early) alongside
/// every other file's result.
pub fn read_raw_worldgen_json(
    worldgen_json_dir: &std::path::Path,
    reports_dir: &std::path::Path,
) -> Result<compile::RawWorldgenJson, Vec<super::error::CompileError>>;
```

### `xtask/src/worldgen_data/mod.rs`

```rust
pub mod common; // re-exported from schema:: below for compile.rs's `use schema::*`
pub mod compile;
pub mod error;
pub mod extract;
pub mod intern;
pub mod schema {
    pub mod biome;
    pub mod common;
    pub mod density_function;
    pub mod feature;
    pub mod noise_settings;
    pub mod structure;
    pub use biome::*;
    pub use common::*;
    pub use density_function::*;
    pub use feature::*;
    pub use noise_settings::*;
    pub use structure::*;
}
```

### `xtask/src/lib.rs` (modify — one re-export, alongside M0-B07's existing ones)

```rust
pub mod worldgen_data;
```

### `xtask/src/main.rs` (modify — extend `Command`, unchanged variants elided)

```rust
#[derive(clap::Subcommand, Debug, PartialEq)]
pub enum Command {
    // ...FmtCheck | Lint | LintDeps | Test | FetchData{..} | Codegen{..} | VerifyGenerated (M0-B07)...
    /// GEN-D7: unzip data/minecraft/{worldgen,dimension,dimension_type}/** from the
    /// pinned version's server.jar, plus reports/biome_parameters/** via run_data_reports.
    FetchWorldgenData {
        version: String,
        #[arg(long)]
        server_jar: Option<std::path::PathBuf>,
        #[arg(long)]
        offline: bool,
    },
    /// GEN-D9: read a prior FetchWorldgenData run's cached JSON and emit
    /// crates/worldgen/generated/v<protocol_version>/{data.postcard, MANIFEST.json}.
    CompileWorldgenData {
        #[arg(long, default_value = "26.2")]
        version: String,
        #[arg(long, default_value_t = 776)]
        protocol_version: u32,
    },
}
```

### `crates/worldgen/src/data/types.rs`

The compiled, id-interned twin of the `xtask` schema above — every `DensityFunctionRef`
collapses to a plain `DensityFunctionId`; every named cross-family reference
(`noise: String` -> `NoiseParamId`, `feature: String` -> `ConfiguredFeatureId`, …) is
resolved. Every named structural family below carries the **same field shape** as its
`xtask` schema twin (Deliverables above), with the substitutions in this table — the
signatures are not repeated a second time in full; only the shape of the substitution
and the two families requiring genuinely new (not just id-substituted) types are shown.

| xtask schema type | Compiled twin | Substitution |
|---|---|---|
| `DensityFunctionRef` | `DensityFunctionId(pub u32)` | flattened: no `Number`/`Reference`/`Inline` — every `Constant`/named-reference/nested-object resolves to one graph-table index |
| `DensityFunctionJson` (34 variants) | `DensityFunctionNode` (same 34 variants) | every `DensityFunctionRef` field -> `DensityFunctionId`; every noise-name field (`Noise`/`ShiftedNoise`'s `noise: String`, `ShiftA`/`ShiftB`/`Shift`'s `argument: String`) -> `NoiseParamId` |
| `SplineJson`/`SplinePointJson` | `Spline`/`SplinePoint` | `coordinate: DensityFunctionRef` -> `DensityFunctionId` |
| `NoiseParamsJson` | `NoiseParams` | unchanged (leaf data, no references) |
| `NoiseRouterJson` | `NoiseRouter` | every field -> `DensityFunctionId` |
| `NoiseGeneratorSettingsJson` | `NoiseGeneratorSettings` | `noise_router: NoiseRouterJson` -> `NoiseRouter`; `surface_rule` -> `SurfaceRule`; `spawn_target: Vec<BiomeParameterPointJson>` -> `Vec<QuantizedClimatePoint>` (below) |
| `SurfaceRuleJson`/`SurfaceConditionJson` | `SurfaceRule`/`SurfaceCondition` | `noise: String` field (in `NoiseThreshold`) -> `NoiseParamId` |
| `ConfiguredCarverJson` | `ConfiguredCarver` | unchanged shape (no cross-references beyond its own leaf `BlockStateSpec`s) |
| `ConfiguredFeatureJson` | `ConfiguredFeature` | unchanged (`config` stays `serde_json::Value`, Context) |
| `PlacedFeatureJson` | `PlacedFeature` | `feature: String` -> `ConfiguredFeatureId` |
| `PlacementModifierJson` (15 variants) | `PlacementModifier` (same 15 variants) | unchanged (no cross-family references) |
| `StructureSetJson` | `StructureSet` | `structures: Vec<StructureSelectionEntryJson>` -> each `structure: String` -> `StructureId` |
| `StructureJson` | `Structure` | unchanged shape (`extra` stays `serde_json::Value`, Context) |
| `TemplatePoolJson`/`PoolElementJson` | `TemplatePool`/`PoolElement` | `fallback: String` -> `TemplatePoolId`; `location: String` (single/legacy-single) -> `ResourceLocation` (template NBT id, left unresolved — GEN-D23, no NBT is ever read by this blueprint); `feature: String` (feature pool element) -> `ConfiguredFeatureId`; `processors: Option<String>` -> `Option<ProcessorListId>` |
| `ProcessorListJson`/`StructureProcessorJson`/`RuleTestJson`/`PosRuleTestJson` | `ProcessorList`/`StructureProcessor`/`RuleTest`/`PosRuleTest` | unchanged (no cross-family references beyond leaf `BlockStateSpec`) |
| `BiomeParameterEntryJson` | n/a — folded into `BiomeParameterList` (below) | quantized (GEN-D14) |

New types with no direct schema twin:

```rust
use std::collections::BTreeMap;
// `ResourceLocation` (this blueprint's own compiled-side copy, identical shape to the
// `xtask` schema twin) is defined further down in this same file, alongside every
// other family-shaped type below — no separate import needed.

macro_rules! interned_id { ($name:ident) => {
    #[derive(serde::Serialize, serde::Deserialize, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct $name(pub u32);
}}
interned_id!(DensityFunctionId);
interned_id!(NoiseParamId);
interned_id!(ConfiguredFeatureId);
interned_id!(StructureId);
interned_id!(TemplatePoolId);
interned_id!(ProcessorListId);

/// The interned density-function forest: every `Constant`/named/inline node across
/// every JSON family becomes one entry here, referenced everywhere else by id. `names`
/// maps every file-level named entry (`"minecraft:overworld/continents"`, …) to its
/// root id; not every id in `nodes` has a name (inline/anonymous sub-nodes do not).
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct DensityFunctionGraph {
    pub nodes: Vec<DensityFunctionNode>,
    pub names: BTreeMap<String, DensityFunctionId>,
}
impl DensityFunctionGraph {
    pub fn get(&self, id: DensityFunctionId) -> &DensityFunctionNode { &self.nodes[id.0 as usize] }
    pub fn resolve(&self, name: &str) -> Option<DensityFunctionId> { self.names.get(name).copied() }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct NoiseParamTable {
    pub params: Vec<NoiseParams>,
    pub names: BTreeMap<String, NoiseParamId>,
}

/// Vanilla's `Climate.Parameter`-equivalent: `(f32 * 10000.0) as i64`, truncated toward
/// zero (GEN-D14, resolved — confirmed exact formula and truncation direction,
/// `docs/research/mc-26.2/17-noise-math.md §3.9`). A scalar point quantizes to
/// `(v, v)`; a `[min, max]` span quantizes each bound independently.
pub fn quantize_climate(v: f32) -> i64 { (v * 10000.0) as i64 }

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy)]
pub struct QuantizedSpan { pub min: i64, pub max: i64 }

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy)]
pub struct QuantizedClimatePoint {
    pub temperature: QuantizedSpan,
    pub humidity: QuantizedSpan,
    pub continentalness: QuantizedSpan,
    pub erosion: QuantizedSpan,
    pub depth: QuantizedSpan,
    pub weirdness: QuantizedSpan,
    pub offset: i64, // quantize_climate(offset.unwrap_or(0.0))
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct BiomeParameterList {
    /// Parallel to a flattened `Climate.ParameterList` (GEN-D14) — this blueprint stores
    /// the flat, quantized list; the R-tree nearest-neighbor search structure itself is
    /// an evaluation-layer (M5-B03+) performance detail built on top of this table, per
    /// GEN-D14's own "search implementation is a pure performance choice" note.
    pub entries: Vec<(QuantizedClimatePoint, ResourceLocation)>, // (point, biome id)
}

/// The compiled blob's root — everything `crates/worldgen/generated/v776/data.postcard`
/// holds (GEN-D9). `PartialEq` is required (and derivable throughout every field type
/// transitively) by `worldgen_compile_determinism.rs`/`data_loader.rs`'s equality
/// assertions (Acceptance tests) — every compiled type in this module derives it.
///
/// Four families get their own interned integer id (`ConfiguredFeatureId`/`StructureId`/
/// `TemplatePoolId`/`ProcessorListId`, each with a companion `..._names` index below)
/// because something *else* in this same compiled graph references them by name and
/// needs a validated, resolved handle back (`placed_feature.feature`, `structure_
/// selection_entry.structure`, `pool_element.processors`/`feature`, `template_pool.
/// fallback`). The other five families (`noise_generator_settings`, `configured_
/// carvers`, `placed_features`, `structure_sets`, `biome_parameter_lists`) are never
/// referenced by id from anywhere else in this blueprint's schema — nothing in the ten
/// JSON node families this blueprint owns points *at* a `noise_generator_settings`,
/// `configured_carver`, `placed_feature`, `structure_set`, or biome-parameter-list
/// entry — so they stay looked up directly by `ResourceLocation`, exactly as
/// `density_functions`/`noise_params` already are via `DensityFunctionGraph.names`/
/// `NoiseParamTable.names`.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct WorldgenData {
    pub protocol_version: u32,
    pub mc_version: String,
    pub density_functions: DensityFunctionGraph,
    pub noise_params: NoiseParamTable,
    pub noise_generator_settings: BTreeMap<ResourceLocation, NoiseGeneratorSettings>,
    /// Keyed by dimension family id (`"minecraft:overworld"`, `"minecraft:nether"`).
    pub biome_parameter_lists: BTreeMap<ResourceLocation, BiomeParameterList>,
    pub configured_carvers: BTreeMap<ResourceLocation, ConfiguredCarver>,
    pub configured_features: BTreeMap<ConfiguredFeatureId, ConfiguredFeature>,
    pub configured_feature_names: BTreeMap<String, ConfiguredFeatureId>,
    pub placed_features: BTreeMap<ResourceLocation, PlacedFeature>,
    pub structure_sets: BTreeMap<ResourceLocation, StructureSet>,
    pub structures: BTreeMap<StructureId, Structure>,
    pub structure_names: BTreeMap<String, StructureId>,
    pub template_pools: BTreeMap<TemplatePoolId, TemplatePool>,
    pub template_pool_names: BTreeMap<String, TemplatePoolId>,
    pub processor_lists: BTreeMap<ProcessorListId, ProcessorList>,
    pub processor_list_names: BTreeMap<String, ProcessorListId>,
}
```

(Every remaining compiled type — `DensityFunctionNode`, `Spline`/`SplinePoint`,
`NoiseParams`, `NoiseRouter`, `NoiseGeneratorSettings`, `SurfaceRule`/`SurfaceCondition`,
`ConfiguredCarver`, `ConfiguredFeature`, `PlacedFeature`, `PlacementModifier`,
`StructureSet`/`StructureSelectionEntry`/`StructurePlacement`, `Structure`,
`TemplatePool`/`PoolElement`, `ProcessorList`/`StructureProcessor`/`RuleTest`/
`PosRuleTest`, `ResourceLocation`, `TagOrList`, `BlockStateSpec`, `HeightProviderJson`-
twin `HeightProvider`, `VerticalAnchor`, `IntProvider` — plus the small enums
`DecorationStep`/`TerrainAdaptation`/`RandomSpreadType`/`Projection`/`StoneDepthSurfaceType`
— is declared with the identical field list as its `xtask/src/worldgen_data/schema/*.rs`
twin per the substitution table above, `#[derive(Serialize, Deserialize, Debug, Clone,
PartialEq)]` in place of `#[derive(Deserialize)]`, and no `deny_unknown_fields`/
`untagged` attributes — those are parse-time-only concerns; the compiled side is always
written by `compile()`, never hand-authored or externally deserialized from untrusted
JSON. `untagged` enums on the schema side (`DensityFunctionRef`, `TagOrList`,
`ClimateSpanJson`, `SplineJson`, `IntProviderJson`) do not need a compiled-side
`#[serde(...)]` attribute at all, tagged or otherwise — `serde`'s default (adjacently-
tagged-by-variant-name) representation is fine for a type only ever written and read by
this project's own `postcard` round-trip.)

### `crates/worldgen/src/data/loader.rs`

```rust
use super::types::WorldgenData;

#[derive(Debug)]
pub enum WorldgenLoadError { Deserialize(String) }
impl std::fmt::Display for WorldgenLoadError { /* ... */ }
impl std::error::Error for WorldgenLoadError {}

/// Pure, allocation-only `postcard` decode — no filesystem/static access, so it is
/// directly unit-testable against a synthetic blob (Acceptance tests).
pub fn parse(bytes: &[u8]) -> Result<WorldgenData, WorldgenLoadError>;

static COMPILED: std::sync::OnceLock<WorldgenData> = std::sync::OnceLock::new();

/// GEN-D9's exact mechanism: `include_bytes!` the committed blob, deserialize once
/// behind a `OnceLock`. Panics (with `WorldgenLoadError`'s message) on a decode
/// failure — the committed blob is this project's own build artifact, so a failure here
/// is an internal invariant violation (a stale/corrupted commit), never a
/// runtime-recoverable condition; callers needing a `Result` use `parse` directly
/// against their own bytes instead.
pub fn load() -> &'static WorldgenData {
    COMPILED.get_or_init(|| {
        parse(include_bytes!("../../generated/v776/data.postcard"))
            .expect("crates/worldgen/generated/v776/data.postcard failed to decode — regenerate via `cargo xtask compile-worldgen-data`")
    })
}
```

### `crates/worldgen/src/data/mod.rs`

```rust
pub mod loader;
pub mod types;
pub use loader::{load, parse, WorldgenLoadError};
pub use types::*;
```

### `crates/worldgen/src/lib.rs` (modify — one new top-level module)

```rust
pub mod data;
```

### `crates/worldgen/generated/v776/.gitkeep` (placeholder until the manual step, Implementation step 13, produces the real files)

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary** (TEST-D45/D46, restated): the test files below plus every
`xtask/src/worldgen_data/**` and `crates/worldgen/src/data/**` file from Deliverables —
every function body `todo!()`-stubbed, every struct/enum field/derive exactly as
declared — are committed first. The implementation changeset (Implementation steps)
fills in real bodies only; it must not edit any test file below, and must not touch
`crates/worldgen/generated/v776/*` (that directory's real content is Implementation
step 13's separate, manual, jar-gated step, exactly mirroring M0-B07's own step 11).

### `xtask/tests/worldgen_schema_density_function.rs`

1. `parses_constant_and_bare_number_shorthand` — `r#"{"type":"minecraft:constant","argument":1.5}"#` parses to `DensityFunctionJson::Constant{argument: 1.5}`; a `DensityFunctionRef` field fed the bare JSON `2.0` parses to `DensityFunctionRef::Number(2.0)`.
2. `parses_add_with_nested_inline_and_reference_children` — `{"type":"minecraft:add","argument1":{"type":"minecraft:constant","argument":1.0},"argument2":"minecraft:overworld/continents"}` parses; `argument1` is `Inline(Constant{1.0})`, `argument2` is `Reference("minecraft:overworld/continents")`.
3. `parses_spline_multipoint_recursively` — a 2-point `spline` object with one point's `value` itself a nested `Multipoint`; both levels parse, `points.len() == 2`, the nested level's own `points.len()` matches.
4. `parses_interval_select_thresholds_and_branches` — a 2-threshold, 3-branch `interval_select` parses with `thresholds.len() == 2`, `branches.len() == 3`.
5. `unknown_density_function_type_is_a_parse_error` — `{"type":"minecraft:not_a_real_node","argument":1.0}"` fails to deserialize into `DensityFunctionJson` (serde's own tag-not-found error — this is what `read_raw_worldgen_json` turns into `CompileError::UnknownNodeKind`, tested at the `compile` level in a separate test below).
6. `all_34_variants_round_trip_a_minimal_literal` — table-driven: one minimal, syntactically-valid JSON literal per one of the 34 `DensityFunctionJson` variants (constructed by hand, matching each variant's field list above), asserting each parses without error and matches the expected variant discriminant.

### `xtask/tests/worldgen_schema_noise_settings.rs`

1. `parses_noise_router_all_15_slots` — a synthetic `NoiseRouterJson` literal with all 15 slots present, alternating between a bare-number shorthand (`0.0`) and a named-string reference (`"minecraft:zero"`) across different slots to exercise `DensityFunctionRef`'s two non-inline shapes; parses, and every slot resolves to the expected `DensityFunctionRef::Number`/`Reference` variant.
2. `parses_surface_rule_sequence_of_conditions` — a 2-entry `sequence` of `condition`-wrapped `block` rules (one guarded by a `y_above` condition, one by a `biome` condition) parses; `SurfaceRuleJson::Sequence(_).len() == 2`.
3. `parses_noise_generator_settings_full_shape` — a complete synthetic `NoiseGeneratorSettingsJson` (all 11 top-level fields present, minimal nested content) parses with every field populated as expected (`sea_level`, `legacy_random_source`, etc. match the literal's values).

### `xtask/tests/worldgen_schema_biome.rs`

1. `parses_scalar_and_span_climate_values` — one entry with all-scalar `parameters`, one with all-`[min,max]` spans; both parse, `ClimateSpanJson::Point`/`Span` discriminate correctly.
2. `quantize_climate_matches_confirmed_formula` — `rc_worldgen::data::quantize_climate(-0.12345) == -1234` (truncation toward zero, GEN-D14's resolved constant — the exact worked example from Context/17-noise-math.md §3.9); `quantize_climate(0.5) == 5000`; `quantize_climate(-0.00001) == 0` (truncation, not floor, at a negative near-zero value).

### `xtask/tests/worldgen_schema_structure.rs`

1. `parses_structure_set_random_spread` — a `random_spread` placement with `salt`/`spacing`/`separation`/`spread_type` set, `structures` a 2-entry weighted list; all fields match.
2. `parses_structure_set_concentric_rings` — mirrors the stronghold shape from Context (`distance=32, spread=3, count=128, salt=0`); all fields match.
3. `parses_template_pool_weighted_elements` — a `fallback` plus 2 `single_pool_element` entries at different weights; `elements.len() == 2`, each `PoolElementJson::SinglePoolElement` with the expected `location`.
4. `parses_processor_list_rule_processor` — a `processor_list` with one `rule` processor containing one `ProcessorRuleJson` (`block_match` input predicate, `always_true` location predicate, `always_true` position predicate); parses, `rules.len() == 1`.
5. `structure_json_captures_jigsaw_fields_in_extra` — a `structure` literal with `type: "minecraft:jigsaw"` and jigsaw-specific fields (`start_pool`, `size`, `max_distance_from_center`) alongside the common fields; common fields parse into their typed slots, and `extra` contains the three jigsaw-specific keys with their raw JSON values.

### `xtask/tests/worldgen_compile_determinism.rs`

1. `output_is_independent_of_input_insertion_order` — build two logically-identical small `RawWorldgenJson` values (one density function, one noise param, one noise_generator_settings referencing the density function by name), inserted into their `BTreeMap`s in reverse order between variant A and variant B (`BTreeMap` itself is order-independent on `insert`, so this specifically proves `compile`'s own id-assignment logic — Interning rule 2 — not accidentally reintroducing order-sensitivity); assert `compile(&a, 776, "26.2") == compile(&b, 776, "26.2")` (`WorldgenData` derives `PartialEq` for this test — added as a `#[cfg(test)]`-only derive on the compiled types, or via a small `#[derive(PartialEq)]` restricted to the fields this test needs; either is implementer's choice, not a Deliverables-breaking addition).

### `xtask/tests/worldgen_compile_errors.rs`

1. `unknown_node_kind_is_reported_with_file_and_kind` — a `RawWorldgenJson` whose single density-function file fails to parse under `DensityFunctionJson` (constructed via `read_raw_worldgen_json` against a temp-directory fixture containing one malformed file, per test 6 above's item 5) surfaces a `CompileError::ParseError`/`UnknownNodeKind` naming that file's `ResourceLocation`.
2. `dangling_density_function_reference_is_reported` — a `noise_router` slot referencing `"minecraft:does_not_exist"`; `compile` returns `Err` containing exactly one `CompileError::DanglingReference{target: "minecraft:does_not_exist", ..}`.
3. `reference_cycle_is_detected` — two named density-function entries, `a` referencing `b` and `b` referencing `a`; `compile` returns `Err` containing a `CompileError::ReferenceCycle` whose `cycle` contains both `a` and `b`.
4. `interval_select_arity_mismatch_is_reported` — an `interval_select` with 2 thresholds but only 2 branches (should be 3); `compile` returns `Err` containing a `CompileError::ArityMismatch`.
5. `multiple_errors_are_all_collected_not_just_the_first` — a `RawWorldgenJson` containing both a dangling reference and an arity mismatch in unrelated entries; `compile`'s `Err` contains both errors, not just one.

### `xtask/tests/worldgen_compile_golden_shape.rs`

1. `overworld_shaped_settings_compile_to_a_fully_resolved_router` — a synthetic (fabricated small values, never real Mojang numeric content — Constraints (c)) `NoiseGeneratorSettingsJson` shaped like the overworld preset (all 15 `NoiseRouterJson` slots present, `legacy_random_source: false`, `aquifers_enabled: true`, `ore_veins_enabled: true`, `sea_level: 63`, a 2-point `spawn_target`) compiles; every `NoiseRouter` field resolves to a valid `DensityFunctionId` indexing into `density_functions.nodes`, `spawn_target.len() == 2`, and the compiled `NoiseGeneratorSettings`'s scalar fields match the input verbatim.

### `xtask/tests/worldgen_data_cli_parsing.rs`

1. `parses_fetch_worldgen_data_with_version_only` — `Cli::try_parse_from(["xtask","fetch-worldgen-data","26.2"])` matches `Command::FetchWorldgenData{version, server_jar: None, offline: false} if version == "26.2"`.
2. `parses_compile_worldgen_data_with_defaults` — `["xtask","compile-worldgen-data"]` matches `Command::CompileWorldgenData{version, protocol_version: 776} if version == "26.2"`.

### `crates/worldgen/tests/data_loader.rs`

1. `parse_round_trips_a_synthetic_worldgen_data` — build a minimal `WorldgenData` by hand (one density function node, one noise param, empty everything else), `postcard::to_allocvec` it, `loader::parse` the bytes back, assert equality (`WorldgenData: PartialEq`, same test-only derive as `worldgen_compile_determinism.rs`'s item 1).
2. `parse_reports_an_error_on_corrupted_bytes` — `loader::parse(&[0xFF, 0xFF, 0xFF])` returns `Err(WorldgenLoadError::Deserialize(_))`, never panics.

## Implementation steps

1. **`xtask/Cargo.toml` + `crates/worldgen/Cargo.toml`.** Apply the Deliverables edits exactly.
2. **`xtask/src/worldgen_data/schema/common.rs`.** Implement `ResourceLocation::parse`/`as_string` and its `serde(try_from/into = "String")` impls; `BlockStateSpec::try_from(String)`; the `HeightProviderJson` bare-shorthand fallback (a custom `Deserialize` impl: try the tagged-enum form first via `serde_json::Value` re-parse, fall back to `VerticalAnchorJson` on failure — mirrors `DensityFunctionRef`'s own untagged pattern but needs a manual impl since the tagged variant isn't itself untaggable alongside a bare value in one `#[serde(untagged)]` block cleanly). Observable: compiles; `worldgen_schema_density_function.rs`'s bare-number test (via `DensityFunctionRef`, which depends on nothing from this file) already passes independent of this step.
3. **`xtask/src/worldgen_data/schema/density_function.rs`.** Exactly the 34-variant enum and `SplineJson`/`DensityFunctionRef` as declared. Observable: `worldgen_schema_density_function.rs` passes.
4. **`xtask/src/worldgen_data/schema/{noise_settings.rs, biome.rs}`.** As declared; `SpawnTargetPointJson` is a type alias to `biome::BiomeParameterPointJson`, so write `biome.rs` first. Observable: `worldgen_schema_noise_settings.rs`, `worldgen_schema_biome.rs` pass.
5. **`xtask/src/worldgen_data/schema/{feature.rs, structure.rs}`.** As declared. Observable: `worldgen_schema_structure.rs` passes (`worldgen_schema_feature.rs` is not a named test file in this blueprint's own Acceptance tests — feature/placement schema correctness is exercised indirectly via `worldgen_compile_golden_shape.rs` and the density-function-focused tests above; a dedicated feature-schema test file is not required by this blueprint but is not forbidden if an implementer wants extra coverage).
6. **`xtask/src/worldgen_data/error.rs`, `intern.rs`.** As declared.
7. **`crates/worldgen/src/data/types.rs`.** Every compiled type per the substitution table, `derive(Serialize, Deserialize, Clone, Debug)` throughout (plus `PartialEq` — Acceptance tests' note). Observable: `cargo build -p rc-worldgen` compiles.
8. **`crates/worldgen/src/data/loader.rs`, `mod.rs`, `crates/worldgen/src/lib.rs`.** As declared. Observable: `crates/worldgen/tests/data_loader.rs` passes.
9. **`xtask/src/worldgen_data/compile.rs` — `compile`.** Algorithm, in order: (a) intern every `density_functions`/`noise_params` entry via two `Interner`s (Deliverables), assigning ids in ascending `ResourceLocation` order (Context rule 2); (b) recursively lower every `DensityFunctionJson`/`DensityFunctionRef` into `DensityFunctionGraph` entries — `Number(v)` and inline objects allocate a fresh anonymous node each time they're encountered (no dedup across structurally-identical-but-separately-written inline nodes — GEN-D12's memoization is an *evaluation-time* concern over this graph's ids, not a compile-time dedup concern), a `DensityFunctionRef::Reference(name)` looks up `name` in the density-function `Interner`, missing -> `CompileError::DanglingReference`; **separately**, every `Noise`/`OldBlendedNoise`-adjacent node's own `noise`/`argument: String` field (`Noise`, `ShiftedNoise`, `ShiftA`, `ShiftB`, `Shift`) is resolved the identical way but against the **`noise_params` `Interner`** instead, producing a `NoiseParamId` stored on the lowered `DensityFunctionNode`, same `DanglingReference` error shape on a miss — two distinct interners, two distinct id types, the same lookup-and-report pattern; (c) after every named entry is lowered, run a cycle check over the *named* subset only (anonymous inline nodes cannot participate in a cycle by construction, since they have no name to be referenced by) via DFS with a recursion-stack set, collecting one `CompileError::ReferenceCycle` per detected cycle; (d) validate `IntervalSelect`'s `thresholds.len() + 1 == branches.len()` per node, `CompileError::ArityMismatch` on mismatch; (e) lower every other family (`noise_generator_settings` — including its own `noise_router`'s 15 `DensityFunctionRef` slots and `surface_rule`'s `noise_threshold` conditions' own `noise: String` field, both resolved exactly as (b)'s two-interner pattern — plus `spawn_target`'s climate points via `quantize_climate`, Deliverables; `configured_carvers`; `configured_features` + their own id interner; `placed_features` resolving `feature: String` against that interner; `structure_sets`; `structures` + interner; `template_pools` + interner resolving `fallback`/`feature`/`processors` references; `processor_lists` + interner), collecting `DanglingReference` for every unresolved cross-family string exactly as step (b) does; (f) if the collected error `Vec` is non-empty, return `Err`; else assemble and return the full `WorldgenData`. Observable: `worldgen_compile_determinism.rs`, `worldgen_compile_errors.rs`, `worldgen_compile_golden_shape.rs` all pass.
10. **`xtask/src/worldgen_data/extract.rs` — `read_raw_worldgen_json`.** Walk each family's directory recursively (`walkdir`-free — plain recursive `std::fs::read_dir`, no new dependency), deriving each file's `ResourceLocation` from its path relative to the family root (Deliverables' doc comment gives the exact rule); `serde_json::from_str` each file under its family's schema type, collecting `CompileError::ParseError`/`Io` per failure rather than stopping. Observable: exercised by `worldgen_compile_errors.rs`'s item 1 (via a small temp-directory fixture) and, implicitly, by the manual step below.
11. **`xtask/src/worldgen_data/extract.rs` — `run`.** Mirrors M0-B07's `fetch.rs::run` exactly for jar acquisition (steps (a)-(e) of that function, reused verbatim in structure — `--offline`/`--server-jar` handling identical): call `crate::fetch_data::fetch_server_jar`/construct a `FetchedJar` under `--offline` exactly as M0-B07 does. Then: (f) call `crate::fetch_data::run_data_reports` (for `biome_parameters/**`, reused, not reinvoked a second time if M0-B07's own `fetch-data` already populated the same cache — `run_data_reports` is idempotent, Context); (g) open the jar via `zip::ZipArchive`, iterate every entry whose name starts with `data/minecraft/worldgen/`, `data/minecraft/dimension/`, or `data/minecraft/dimension_type/`, and extract each to the identical relative path under `worldgen_json_dir` (create parent dirs as needed); never touch any entry under `data/minecraft/structure/` (GEN-D23 — no matching prefix in the extraction filter above, so this is enforced by construction, not a separate check). Observable: exercised only by the manual verification procedure (needs a real jar).
12. **`xtask/src/main.rs` dispatch + `worldgen_data`/`fixture_manifest` wiring for `CompileWorldgenData`.** `FetchWorldgenData{..}` -> `worldgen_data::extract::run`. `CompileWorldgenData{..}` -> `worldgen_data::extract::read_raw_worldgen_json` then `worldgen_data::compile::compile`; on success, `postcard::to_allocvec(&data)`, write `crates/worldgen/generated/v{protocol_version}/data.postcard`, build the manifest via the **reused** `xtask::fixture_manifest::build_manifest` (M0-B07, unmodified) over `[("data.postcard", bytes)]`, write `MANIFEST.json`, immediately self-verify via `fixture_manifest::verify_manifest` exactly as `codegen::run` does. `VerifyGenerated` (M0-B07's existing verb) is extended to additionally check `crates/worldgen/generated/v776/MANIFEST.json` alongside its existing `crates/registries/generated/v776/MANIFEST.json` check — both are ordinary `verify_manifest` calls against different paths, no new logic. Observable: full automated test suite green.
13. **(Manual, requires a legal jar — not part of this blueprint's own CI-checkable Done state, mirrors M0-B07's step 11.)** `cargo xtask fetch-worldgen-data 26.2` then `cargo xtask compile-worldgen-data`; confirm `crates/worldgen/generated/v776/{data.postcard, MANIFEST.json}` exist and `cargo xtask verify-generated` exits 0; confirm `cargo build -p rc-worldgen` succeeds with the real blob now `include_bytes!`'d (replacing the `.gitkeep` placeholder — delete it once real files exist); confirm every moderate-confidence field name (Context) round-tripped without a `CompileError::ParseError`/`UnknownNodeKind` — any such error at this step means a field-name reconciliation is needed before committing, exactly the self-checking property Context promises. Commit the three resulting files (`data.postcard`, `MANIFEST.json`, and the removal of `.gitkeep`) in their own changeset (GEN-D24/ASSET-D25).

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under Acceptance tests is committed first, with every Deliverables function body `todo!()`-stubbed. The implementation changeset (steps 1-12) fills in real bodies only; it must not edit any test file, and must not touch `crates/worldgen/generated/v776/*` (step 13 is a separate, manually-performed step outside both changesets, gated on legal jar access).

(b) **No new external dependencies beyond the pinned set — zero new `[workspace.dependencies]` pins.** `zip` (new consumer: `xtask`) and `serde`/`serde_json`/`postcard` (new consumer: `rc-worldgen`) are all already version-pinned in `12-workspace-structure.md`'s `[workspace.dependencies]` table; this blueprint's two `Cargo.toml` edits (Deliverables) only add new `<crate>.workspace = true` consumer lines, never a new version string anywhere. The one new internal (path, not external) dependency edge — `xtask` -> `rc-worldgen` — is likewise not a new external pin. Do not add `walkdir`, `anyhow`, `regex`, or any other crate not named in Deliverables.

(c) **No Mojang or third-party reimplementation code, and no real Mojang numeric content in the automated test suite.** Every JSON shape parsed by this blueprint's schema is derived from `docs/research/mc-26.2/{05-worldgen.md, 06-structures.md, 17-noise-math.md}` (ASSET-D18/D30-produced research) and the public Mojang worldgen JSON schema (ASSET-D18(b)/(e)) — no decompiled source, no third-party reimplementation's code, is consulted or copied while writing any file this blueprint creates. Every literal used inside `xtask/tests/**`/`crates/worldgen/tests/**` is a small, fabricated, structurally-representative value (as M0-B07's own test literals are) — never a real vanilla noise/spline/spacing constant copied out of a real `server.jar` or the ASSET-D18(f) reference.

(d) **Unknown-field/unknown-node policy is binding, not a suggestion.** Every schema struct in Deliverables (except the intentionally-generic `serde_json::Value` payload fields, Context) carries `#[serde(deny_unknown_fields)]` or is a `#[serde(tag = "...")]` enum (which rejects an unrecognized tag value by construction); an implementer must not relax either mechanism, since a mismatch is this blueprint's designed signal that a field name needs reconciliation (Context), not a warning to be silently swallowed.

(e) **`compile()` never touches structure NBT template bytes.** No file this blueprint's `extract.rs` reads or `compile.rs` processes is ever a `.nbt` file; template `location` strings (inside `PoolElementJson`/`PoolElement`) are stored as opaque `ResourceLocation` values, never resolved to file content (GEN-D23/ASSET-D16 — that resolution is a later blueprint's runtime, operator-supplied-path concern).

(f) **No `unsafe` code.** Every function in this blueprint's deliverables is implementable in 100% safe Rust.

(g) **Density-function/noise/surface-rule/carver/feature/structure evaluation is out of scope, full stop.** No function in this blueprint's deliverables computes a noise value, evaluates a spline, walks a surface-rule tree against a real column, places a feature, or assembles a jigsaw structure. `DensityFunctionNode`/`SurfaceRule`/`PlacementModifier`/etc. are pure data; adding an `impl` block with evaluation logic anywhere in `crates/worldgen/src/data/` is out of this blueprint's scope even if it would compile cleanly.

## Verification commands

Automated, run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43) — no jar, no network, no local Java required:

```
cargo build -p xtask --all-features
cargo build -p rc-worldgen --all-features
cargo nextest run -p xtask
cargo nextest run -p rc-worldgen
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p xtask` includes all nine of this blueprint's new `xtask` test files alongside M0-B07's/M0-B01's existing `xtask` suites, all green; `cargo nextest run -p rc-worldgen` includes `data_loader.rs`'s two cases.

Manual, requires a locally supplied or network-fetchable legal Minecraft 26.2 `server.jar` and a local Java 25+ runtime (never run by CI in this blueprint's own gate):

```
cargo xtask fetch-worldgen-data 26.2
cargo xtask compile-worldgen-data
cargo xtask verify-generated
cargo build -p rc-worldgen --all-features
```

Expected: every command exits 0; `crates/worldgen/generated/v776/{data.postcard, MANIFEST.json}` exist and, once committed, are what all future ordinary CI runs build against — with no further jar or network dependency from that point on.
