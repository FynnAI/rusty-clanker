# M0-B07 — xtask Version-Data Pipeline (fetch-data / codegen)

| Field | Content |
|---|---|
| ID | M0-B07 |
| Milestone | M0 — Engine Skeleton & Workspace Bootstrap |
| Prerequisites | M0-B01 (workspace scaffold: root `Cargo.toml`, `xtask/` with its `Cli`/`Command` surface and `xtask/src/lib.rs` re-exports already exist; the `crates/registries/generated/.gitkeep` placeholder this blueprint's output lands under already exists); M0-B08 (`xtask/src/fetch_data.rs`: `fetch_server_jar`/`run_data_reports`, the single, authoritative, shared home of the NET-D9 jar-fetch/`--reports` primitive — this blueprint's own `fetch-data`/`codegen` verbs call it directly rather than re-implementing piston-meta resolution, jar download, or `--reports` invocation a second time; also brings `.gitignore`'s `/oracle/` and `/datagen-output/` entries, which this blueprint reuses as-is) |
| Implements | NET-D9 (version-data pipeline: `fetch-data`/`codegen`), NET-D10 (commit-vs-regenerate boundary), ASSET-D15 (binding confirmation of NET-D10), ASSET-D25 (functional-fact-vs-creative-expression test, applied here to registry/block-state ID data), ASSET-D28(i) (cited as reinforcing precedent: `--reports`-sourced factual measurements are safe to commit as generated data), TEST-D47 (fixture integrity manifest, restated concretely for this blueprint's generated artifacts); satisfies M0's roadmap Acceptance Criterion 3 (`11-roadmap-milestones.md`) |
| Crates touched | `xtask/` (new `fetch-data`/`codegen`/`verify-generated` verbs and supporting modules, reusing M0-B08's already-present `fetch_data` module — no `.gitignore` change, that is M0-B08's); `crates/registries/` (its `generated/v776/` output directory only, per WS-D13 — this blueprint does not modify `crates/registries/src/lib.rs`, `Cargo.toml`, or any other crate's manifest) |
| Estimated scope | L |

## Goal & Done definition

Give `xtask` two new verbs, `fetch-data <version>` and `codegen`, that together implement NET-D9's version-data pipeline: `fetch-data` resolves a Minecraft version against Mojang's public `piston-meta` manifest, downloads (or accepts a locally-supplied) `server.jar`, verifies its SHA-1, confirms a locally installed Java meets the pinned version's declared JVM requirement, and runs the jar's `--reports` data generator; `codegen` reads the cached `--reports` output and emits deterministic, compiling Rust registry/block-state ID tables under `crates/registries/generated/v776/` (WS-D13), alongside a TEST-D47-shaped fixture manifest. A third verb, `verify-generated`, recomputes that manifest's hashes against the files on disk and fails on any mismatch. None of `xtask`'s new *logic* requires a real `server.jar` to be exercised in ordinary CI — every pure parsing/codegen/manifest function is tested against synthetic fixtures shaped exactly like the real reports, and the generated Rust's *compilability* is proven with a standalone `rustc` invocation against synthetically-generated output. Only the one-time act of running the real pipeline against a legally-obtained jar (which produces the actual committed `crates/registries/generated/v776/*.rs` content) needs that jar — exactly the "local, on-demand developer step... never a network fetch performed by CI" NET-D9 already specifies.

Done when:

- [ ] `cargo build -p xtask --all-features` succeeds with zero warnings, including the new `datagen`/`fixture_manifest` modules.
- [ ] Every test in this blueprint's own test changeset (`xtask/tests/datagen_java_check.rs`, `datagen_reports_parsing.rs`, `datagen_codegen.rs`, `fixture_manifest.rs`, `datagen_cli_parsing.rs`) passes under `cargo nextest run -p xtask`, using only synthetic/in-memory fixture data — no real `server.jar`, no network access, no local Java installation required to go green.
- [ ] `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` (M0-B01's four gates) all still exit 0, unaffected by this blueprint's additions.
- [ ] Separately, and not part of this blueprint's own CI-checkable gate: given a locally supplied, legally obtained Minecraft 26.2 `server.jar` (or network access for `fetch-data` to download one itself) and a local Java 25+ runtime, `cargo xtask fetch-data 26.2` followed by `cargo xtask codegen` produces `crates/registries/generated/v776/{registries.rs, block_states.rs, MANIFEST.json}`; `cargo xtask verify-generated` exits 0 against that output; and both generated `.rs` files compile standalone via `rustc --edition 2024 --crate-type lib`. This is M0's own roadmap Acceptance Criterion 3, performed once by whoever has legal access to the jar — CI never performs this step in this blueprint's own Tier-1 gate (NET-D9: "never a network fetch performed by CI").
- [ ] CI tier: Tier 1 (`fmt-check`, `lint`, `lint-deps`, `test` — M0-B01's four gates) green on both `ubuntu-24.04` and `windows-2025` for this blueprint's entire automated changeset.

## Context (self-contained)

### The NET-D9 pipeline, restated exactly

NET-D9's exact text: `xtask fetch-data <version>` "resolves the version against Mojang's public `piston-meta` `version_manifest_v2.json`, downloads the matching `server.jar`, runs `java -DbundlerMainClass=net.minecraft.data.Main -jar server.jar --reports` locally to obtain packet-ID, registry, block-state, and item-ID JSON, and merges that against an in-repo, hand-maintained packet **field-layout** spec (versioned RON/TOML under `crates/protocol/spec/`, authored from minecraft.wiki and packet captures — `--reports` gives IDs and names but never field layouts). `xtask codegen` then emits generated Rust into two homes (`12-workspace-structure.md`'s WS-D13): packet code under `crates/protocol/generated/<protocol-version>/` using `rc-protocol-macros`, and registry/block-state ID tables under `crates/registries/generated/<protocol-version>/` in `rc-registries`, so crates barred from an `rc-protocol` dependency (WS-D3 rule 2) can still consume them. This is a local, on-demand developer step run against a `server.jar` the developer legally downloaded themselves — never a network fetch performed by CI or at build time in a released artifact."

This blueprint implements `fetch-data` in full. It implements `codegen` for the subset of NET-D9's scope that needs **no** hand-authored spec merge: registry entries and block-state IDs are pure numeric/name facts already complete in `--reports` output (`registries.json`, `blocks.json`) — NET-D9 itself draws this distinction ("`--reports` gives IDs and names but never field layouts" is specifically a **packet**-codec problem: a packet's *field layout* — which bytes, in what order, of what type — is never present in `--reports`, but a registry entry's *identity* — its name and protocol-assigned integer id — is the entirety of what a registry-table needs, with nothing left for a hand-authored spec to add).

> **Resolved scope boundary.** Packet-body codegen (`#[derive(RcPacket)]` structs merging `--reports`' `packets.json` with a hand-authored `crates/protocol/spec/*.ron` field-layout spec, consumed by `rc-protocol-macros`) is **out of scope for this blueprint** and lands with the M1 protocol-bootstrap blueprint, for two concrete reasons: (1) `rc-protocol-macros` is still M0-B01's empty-shell proc-macro crate with zero dependencies and zero `#[proc_macro_derive]` items — there is no macro to drive yet; (2) `crates/protocol/spec/*.ron` does not exist yet — no field-layout spec has been authored, and authoring one is squarely M1's scope ("`rc-protocol` + `rc-protocol-macros`: framing/compression..." per `11-roadmap-milestones.md`'s M1 Scope), not M0's. M0's own roadmap Acceptance Criterion 3 only requires "compiling generated code under `crates/registries/generated/v776/`" — it does not require packet definitions specifically — so registry/block-state ID-table codegen fully satisfies the literal criterion without overreaching into M1's territory.
>
> **Output location, resolved (WS-D13).** `12-workspace-structure.md`'s WS-D13 fixes `crates/registries/generated/<protocol-version>/` — inside `rc-registries`, the one crate every registry-data consumer (`rc-protocol` itself included) may depend on — as the single home of generated registry/block-state tables; `crates/protocol/generated/` holds only NET-D9's packet codegen. This blueprint's output therefore lands at `crates/registries/generated/v776/`, the exact path M0's roadmap acceptance criterion names, byte for byte. This blueprint still does not wire the files into `rc-registries`' compiled module tree (that is M1-B05's job, mirroring how packet codegen is a later blueprint's), and the eventual shape of `rc-registries`' full canonical registry types (biomes, entity types, item components, etc.) remains that crate's own later blueprint's call — this blueprint's output is explicitly a minimal M0-scoped table set, not a claim about `rc-registries`' eventual architecture.

### Commit-vs-regenerate boundary (NET-D10, ASSET-D15/D25/D28) — restated exactly

NET-D10, confirmed binding unchanged by ASSET-D15: "the hand-authored packet field-layout spec... is committed. Derived *numeric/structural facts* from `--reports` (packet IDs, registry entry ID↔name tables, block-state ID tables) are committed only as **processed, code-generated Rust source** — never as raw Mojang JSON — to keep builds reproducible offline while treating those tables as functional/factual data rather than copied creative expression. Mojang's `server.jar`, its raw `--reports` output, and any extracted game assets... are **never** committed and never distributed; they exist transiently only on a developer machine that legally obtained the jar."

Applied concretely to this blueprint's own artifacts:

| Artifact | May be committed? | Why |
|---|---|---|
| `server.jar` (downloaded or supplied) | **Never.** Cached under `.gitignore`d `oracle/<version>/server.jar` only — M0-B08's shared `fetch_data::fetch_server_jar` path, reused as-is (see Prerequisites). | NET-D10/ASSET-D13's "ship no Mojang assets" rule; the jar is Mojang's own compiled binary. |
| Raw `datagen-output/<version>/generated/reports/{registries,blocks}.json` | **Never.** Stays in the same `.gitignore`d output directory (M0-B08's `fetch_data::run_data_reports` path, reused as-is). | NET-D10: raw `--reports` JSON is exactly the thing that must never be committed, even though its *content* is factual. |
| `crates/registries/generated/v776/{registries.rs, block_states.rs}` | **Yes, committed.** | NET-D10's carve-out: processed, code-generated Rust source derived from functional facts (protocol-assigned integer IDs, registered names) is not creative expression — the same "functional fact vs. Mojang creative expression" test ASSET-D25 applies to worldgen JSON (GEN-D24) and ASSET-D28(i) applies to `--reports`-sourced hitbox measurements applies identically here: a block's registered name and its protocol-assigned integer id are measurements of Mojang's own registration order, not authored prose or art. |
| `crates/registries/generated/v776/MANIFEST.json` (this blueprint's TEST-D47 manifest) | **Yes, committed.** | It records hashes/provenance of the already-committed generated `.rs` files — itself derived, non-creative metadata, the same category as the `.rs` files it describes. |

### Java 25 requirement — restated exactly, with why it is read dynamically, not hardcoded

`docs/research/mc-26.2/00-source-overview.md` (§5, Constants & magic values, sourced from the pinned version's own `version.json`): `java_version` (required JVM) = **25** (component `java-runtime-epsilon`). This is Mojang's own declared minimum JVM major version for running Minecraft 26.2's `server.jar` — including its `--reports` data-generator entry point, which runs inside the same JVM process as the ordinary server.

This blueprint's `fetch-data` verb reads this requirement **dynamically** — via `fetch_data::FetchedJar::min_java_major`, populated by M0-B08's shared `fetch_server_jar` from the same per-version manifest it already fetches to learn the jar's download URL and SHA-1 (see above) — it does not hardcode `25` as the check's source of truth, because the requirement is itself an authored-per-release Mojang value with no guaranteed stability across future version bumps (NET-D2's "a version bump is a deliberate, reviewed event"). `25` is used only as a **documented fallback constant** (`java_check::FALLBACK_MIN_JAVA_MAJOR`, Deliverables) for the one code path that cannot reach the network at all (`--offline` mode, see above) — with an explicit comment citing this exact research note as its provenance, so a future reader knows why `25` appears as a literal in the source.

### `piston-meta` manifest shapes and the `--reports` invocation — owned by M0-B08's shared `fetch_data.rs`, reused here

NET-D9 names `version_manifest_v2.json` as the top-level resolution source, and pins the exact `--reports` invocation:

```
java -DbundlerMainClass=net.minecraft.data.Main -jar server.jar --reports
```

**This blueprint does not parse either manifest shape or invoke `--reports` itself.** M0-B08's `xtask/src/fetch_data.rs` (Prerequisites) is the single, authoritative home of both — `fetch_data::fetch_server_jar(version_id, repo_root)` resolves `version_manifest_v2.json`, downloads/verifies `server.jar` against the per-version manifest's declared SHA-1, and returns a `FetchedJar { jar_path, version_id, sha1, min_java_major }` (the last field taken from that same per-version manifest's `javaVersion.majorVersion`, exposed specifically so this blueprint never needs its own second manifest fetch just to learn it); `fetch_data::run_data_reports(&jar, repo_root)` runs the exact command above (no `--output` flag — copied verbatim, nothing added) and returns the directory its output landed in. This blueprint's own `fetch.rs` (Deliverables below) is a thin orchestration layer over those two functions — see "`xtask fetch-data` orchestration, reusing M0-B08's shared primitive" below for the offline/`--server-jar` cases those two functions alone do not cover.

This blueprint's `codegen` verb consumes exactly two of the files `run_data_reports`'s output directory contains:

**`reports/registries.json`** — one JSON object per registry (key = registry name, e.g. `"minecraft:block"`), each with an `entries` object mapping every registered entry's namespaced identifier to `{"protocol_id": N}` (`docs/research/mc-26.2/07-blocks-blockstates.md` §7: "`reports/registries.json` → `minecraft:block` | Registration-order-derived `protocol_id` per block"). Illustrative excerpt (values are the true, sourced 26.2 low-numbered entries; the full file has ~100 registries):

```json
{
  "minecraft:block": {
    "default": "minecraft:air",
    "entries": {
      "minecraft:air": { "protocol_id": 0 },
      "minecraft:stone": { "protocol_id": 1 }
    }
  },
  "minecraft:item": {
    "entries": { "minecraft:air": { "protocol_id": 0 } }
  }
}
```

**`reports/blocks.json`** — one JSON object per block (key = namespaced block identifier), each with `definition` (codec name plus construction params — not consumed by this blueprint), `properties` (property name → alphabetically-ordered value list — not consumed by this blueprint), and `states[]`, a list of `{"id": N, "properties": {...}, "default": true|omitted}` entries, exactly one of which per block carries `"default": true`. Illustrative excerpt (`docs/research/mc-26.2/07-blocks-blockstates.md` §3.4: 1196 registered blocks, 32366 total block states for 26.2, ids `0..32365`; `oak_door` alone spans ids `5655..5718`):

```json
{
  "minecraft:air": {
    "definition": { "type": "minecraft:air" },
    "properties": {},
    "states": [ { "id": 0, "default": true } ]
  },
  "minecraft:oak_door": {
    "definition": { "type": "minecraft:door", "block_set_type": "oak" },
    "properties": {
      "facing": ["east", "north", "south", "west"],
      "half": ["lower", "upper"],
      "hinge": ["left", "right"],
      "open": ["false", "true"],
      "powered": ["false", "true"]
    },
    "states": [
      { "id": 5655, "properties": { "facing": "east", "half": "lower", "hinge": "left", "open": "false", "powered": "false" } },
      { "id": 5680, "properties": { "...": "..." }, "default": true },
      { "id": 5718, "properties": { "facing": "west", "half": "upper", "hinge": "right", "open": "true", "powered": "true" } }
    ]
  }
}
```

(`5680` above is an illustrative placeholder position within `oak_door`'s real `5655..=5718` range, not a claimed real value — the real codegen run reads whichever id the actual report flags `"default": true"`.)

### `xtask fetch-data` orchestration, reusing M0-B08's shared primitive

`fetch_data::fetch_server_jar`/`run_data_reports` alone cover the ordinary online path (no locally-supplied jar). This blueprint's own `fetch.rs` (Deliverables below) additionally supports two CLI-level cases NET-D9's own shared primitive does not need to know about, both resolved without duplicating any of `fetch_data.rs`'s own three responsibilities (piston-meta resolution, jar download, `--reports` invocation):

- **`--server-jar <path>` given, online.** The supplied jar's bytes are copied to `fetch_data::ORACLE_JAR_DIR`'s exact expected path (`oracle/<version>/server.jar`) *before* calling `fetch_server_jar` — that function's own already-exists-and-hash-matches fast path (M0-B08's Deliverables) then verifies the supplied jar against Mojang's declared SHA-1 and populates `min_java_major` from the real manifest, with no separate verification path this blueprint would otherwise have to write itself.
- **`--offline` given (requires `--server-jar`).** No network call of any kind is possible, so `fetch_server_jar` is never called (it has no offline mode) — this blueprint instead constructs a `fetch_data::FetchedJar` value directly (every field is `pub`, so this is ordinary struct construction, not a second piston-meta implementation): `jar_path` = the supplied path (or the cache path, if already copied there), `version_id` = the requested version, `sha1` = computed locally via the already-pinned `sha1` crate (no manifest to verify against, so this hash is provenance-only), `min_java_major` = `java_check::FALLBACK_MIN_JAVA_MAJOR` (Deliverables). That value is then passed to `fetch_data::run_data_reports` exactly as the online path would — the `--reports` invocation itself is never duplicated, regardless of which path constructed the `FetchedJar`.

### Determinism (same jar → byte-identical output) — the concrete rule

TEST-D47's manifest scheme only works if a fixture's committed hash is reproducible: re-running `codegen` against the *same* cached reports must produce byte-identical `.rs`/`MANIFEST.json` content every time, with no dependence on incidental JSON key order, `HashMap` iteration order, or wall-clock time. This blueprint fixes the concrete rules an implementer must follow, precisely so this is testable without a real jar:

1. **Parse JSON objects into `BTreeMap<String, _>`, never `HashMap`.** `serde_json` deserializes a JSON object into whatever collection type the target struct field declares; declaring `BTreeMap` guarantees key-sorted iteration for free, with no separate sort step needed for names.
2. **Sort registry entries by `protocol_id` explicitly**, not by name — `protocol_id` order is the meaningful, stable order (registration order), and must be computed via an explicit `sort_by_key` on the collected `(name, id)` pairs, never left to rely on `BTreeMap`'s name-sorted iteration (which would produce a *differently-ordered but still legal* file — legal is not the bar here, byte-identical-across-runs is).
3. **No wall-clock timestamp anywhere in generated output.** Neither the `.rs` files' header comments nor `MANIFEST.json` may embed a generation timestamp, hostname, or any other run-specific, non-reproducible value — the Deliverables' struct/output shapes below have no such field, which is itself the enforcement (an implementer cannot add one without deviating from the pinned shape, forbidden by Constraints below).
4. **Identifier sanitization is a pure function of the input string** (namespace-stripped, case-folded, non-alphanumeric→`_`, leading-digit-guarded, keyword-guarded — exact algorithm in Deliverables) — never influenced by iteration order or position.

Given 1–4, `generate(registries, blocks)` (Deliverables below) is a pure function: identical logical input (regardless of the *order* entries were inserted into the source `BTreeMap`s) produces byte-identical output. This is exactly what this blueprint's `output_is_independent_of_input_insertion_order` acceptance test proves, without needing two real jar runs.

### Why `sha2` is added to `[workspace.dependencies]`

TEST-D47 pins SHA-256 as the fixture-hash algorithm ("SHA-256 of the fixture's own bytes"), but no planning document pins a specific `sha2` crate version — `12-workspace-structure.md`'s table pins `sha1` (`0.11.0`, NET-D6) but not `sha2`. Mirroring M0-B02's own precedent (which added `proptest` to `[workspace.dependencies]` citing TEST-D27's already-established version pin), this blueprint adds one line: `sha2 = "0.11.0"`, matching `sha1`'s already-pinned version exactly, since both are RustCrypto `RustCrypto/hashes`-family crates conventionally released in lockstep within the same version wave — the natural, minimal-risk pairing for a project that already trusts `sha1` at that exact version. This is a cited, deliberate addition (Constraints (b) below), not an invented dependency.

### `xtask`'s existing surface this blueprint extends

Per M0-B01: `xtask/src/main.rs` defines `Cli`/`Command` (re-exported as `xtask::{Cli, Command}` via `xtask/src/lib.rs`), currently with variants `FmtCheck | Lint | LintDeps | Test`. WS-D9's full command surface names `fetch-data <version>` and `codegen` as NET-D9's pipeline, explicitly reserved for a later blueprint by M0-B01's own Constraints — this blueprint is that later blueprint. Per Prerequisites, M0-B08 has already extended `Command` with its own `SetupOracle` (and other) variants and already re-exports `fetch_data` from `xtask/src/lib.rs`; this blueprint adds `FetchData`/`Codegen`/`VerifyGenerated` variants alongside M0-B08's, and adds `datagen` and `fixture_manifest` to `lib.rs`'s re-export list following the identical pattern — `datagen`'s own submodule list no longer includes a `version_manifest` module (that responsibility now lives entirely in M0-B08's `fetch_data.rs`, reused rather than duplicated).

## Deliverables

### `xtask/Cargo.toml` (modify — add two dependencies; M0-B08 has already added `thiserror`/`reqwest`/`sha1` for `fetch_data.rs`, unchanged here)

```toml
[dependencies]
clap = { version = "4.6.6", features = ["derive"] }
xshell = "0.2.7"
serde = { workspace = true }
serde_json = { workspace = true }
sha1 = { workspace = true }
sha2 = { workspace = true }
```

(`clap`/`xshell`/`serde`/`serde_json` lines are M0-B01's, `sha1` is M0-B08's — all unchanged, shown for context. `sha2` is this blueprint's own newly-added line, see Context. This blueprint does **not** add `reqwest` itself: all network access is reused through M0-B08's already-present `fetch_data` module inside this same crate, so no direct `reqwest` usage exists in this blueprint's own deliverables.)

### Root `Cargo.toml` (modify — add one line to `[workspace.dependencies]`)

Add, alphabetically among the existing entries:

```toml
sha2              = "0.11.0"   # TEST-D47; xtask's fixture-manifest SHA-256 hashing
```

(No `.gitignore` change — M0-B08 already reserves `/oracle/` and `/datagen-output/`, Prerequisites.)

### `xtask/src/datagen/java_check.rs`

```rust
//! Local Java-runtime detection (NET-D9's "runs java ... locally" precondition).

/// Documented, well-known Java major-version requirement for Minecraft 26.2's own
/// `server.jar` (docs/research/mc-26.2/00-source-overview.md §5, sourced from the
/// pinned version's own `version.json`, component `java-runtime-epsilon`). Used only as
/// a fallback when `--offline` (Deliverables, `fetch.rs`) prevents constructing a
/// `fetch_data::FetchedJar` with a real, manifest-sourced `min_java_major` — every
/// other code path reads the requirement dynamically off that field, never this
/// constant.
pub const FALLBACK_MIN_JAVA_MAJOR: u32 = 25;

/// Parses the major version number out of `java -version`'s combined stdout+stderr
/// text (the JVM historically writes this to **stderr**; some wrapped/managed JDK
/// launchers redirect to stdout instead — callers concatenate both streams before
/// calling this so stream choice never matters). Handles the modern scheme
/// (`"25"`, `"25.0.1"`) and the legacy `"1.MAJOR.MINOR_PATCH"` scheme used through
/// Java 8 (`"1.8.0_301"` -> `8`). Returns `None` if no quoted version string is found
/// or its leading component does not parse as an integer.
pub fn parse_java_major(version_output: &str) -> Option<u32>;

/// Runs `java -version`, parses its output, and compares against `min_major`.
/// `Ok(detected_major)` if `detected_major >= min_major`. `Err(<actionable message>)`
/// naming exactly one of: `java` not found on `PATH`; output did not contain a
/// parseable version string; or a detected major version below `min_major` (message
/// names both the detected and required values and links to
/// <https://adoptium.net> as a concrete "how do I get one" pointer).
pub fn check_java(min_major: u32) -> Result<u32, String>;
```

### `xtask/src/datagen/reports.rs`

```rust
//! Parsed shapes of the two `--reports` files this blueprint's `codegen` consumes
//! (`registries.json`, `blocks.json`). See Context for the exact JSON shape each
//! mirrors — field names below match the real report's keys verbatim.

use std::collections::BTreeMap;

pub type RegistriesReport = BTreeMap<String, RegistryReport>;

#[derive(serde::Deserialize, Debug, Clone)]
pub struct RegistryReport {
    #[serde(default)]
    pub default: Option<String>,
    pub entries: BTreeMap<String, RegistryEntryReport>,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct RegistryEntryReport {
    pub protocol_id: u32,
}

pub type BlocksReport = BTreeMap<String, BlockReport>;

#[derive(serde::Deserialize, Debug, Clone)]
pub struct BlockReport {
    pub states: Vec<BlockStateReport>,
    // `definition`/`properties` exist in the real report but are not consumed by this
    // blueprint's minimal codegen scope; omitting them from this struct is safe
    // because `#[serde(deny_unknown_fields)]` is deliberately never set here.
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct BlockStateReport {
    pub id: u32,
    #[serde(default)]
    pub default: bool,
}

/// The one state in `block.states` flagged `"default": true`. `None` if none is
/// flagged (a malformed report — every real block has exactly one).
pub fn find_default_state_id(block: &BlockReport) -> Option<u32>;
```

### `xtask/src/datagen/codegen.rs`

```rust
//! Pure codegen (`generate`) plus the CLI-facing I/O wrapper (`run`) for the `codegen`
//! verb. See Context's "Determinism" subsection for the four rules `generate` must
//! follow — restated as doc comments on the function itself below.

use super::reports::{BlocksReport, RegistriesReport};

/// `xtask`'s own crate version, tagged as this codegen format's identity — written
/// into every `MANIFEST.json` entry's `generator_tool_version` field.
pub const CODEGEN_TOOL_VERSION: &str = concat!("xtask-codegen/", env!("CARGO_PKG_VERSION"));

pub struct GeneratedFiles {
    /// `(relative filename under crates/registries/generated/v<protocol_version>/, content)`,
    /// in write order: `("registries.rs", ...)`, `("block_states.rs", ...)`.
    pub files: Vec<(String, String)>,
}

/// Pure transform: `--reports` data in, generated Rust source out. No filesystem
/// access. Deterministic per Context's four rules: parses/iterates only via `BTreeMap`
/// (never `HashMap`), sorts registry entries by `protocol_id` explicitly, embeds no
/// timestamp anywhere, and sanitizes identifiers as a pure function of the input
/// string alone. Two logically-identical `RegistriesReport`/`BlocksReport` values
/// (even if built via `.insert()` calls in different orders) MUST produce byte-
/// identical `GeneratedFiles::files` content — this is the property
/// `output_is_independent_of_input_insertion_order` (Acceptance tests) checks.
pub fn generate(registries: &RegistriesReport, blocks: &BlocksReport) -> GeneratedFiles;

pub struct CodegenArgs {
    /// Directory containing `registries.json`/`blocks.json` (a prior `fetch-data`
    /// run's `datagen-output/<version>/generated/reports/` — M0-B08's shared
    /// `fetch_data::run_data_reports`'s own return path, reused as-is).
    pub reports_dir: std::path::PathBuf,
    /// `crates/registries/generated/v<protocol_version>/` — created if absent.
    pub out_dir: std::path::PathBuf,
    pub source_jar_sha1: String,
    pub protocol_version: u32,
    pub mc_version: String,
}

/// I/O wrapper: reads `registries.json`+`blocks.json` from `args.reports_dir` (`Err`
/// naming the exact missing file and suggesting `cargo xtask fetch-data <version>` if
/// either is absent), calls `generate`, writes both files plus `MANIFEST.json` under
/// `args.out_dir`, then immediately calls `fixture_manifest::verify_manifest` against
/// what it just wrote as a self-check (defense against a write-time bug producing a
/// manifest that does not actually match the bytes on disk).
pub fn run(args: &CodegenArgs) -> Result<(), String>;
```

### `xtask/src/datagen/fetch.rs`

```rust
//! CLI-facing I/O wrapper for the `fetch-data` verb. Reuses M0-B08's shared
//! `crate::fetch_data::{fetch_server_jar, run_data_reports}` for every piece of
//! piston-meta resolution, jar download, and `--reports` invocation (Context) — this
//! file's own job is exactly the two CLI-level cases that shared primitive does not
//! cover (`--server-jar`, `--offline`) plus this verb's own Java-version check and
//! cross-process SHA-1 sidecar persistence.

pub struct FetchArgs {
    pub version: String,
    /// Use this already-downloaded jar instead of letting `fetch_server_jar`
    /// download one. When not `offline`, its bytes are copied into
    /// `fetch_data::ORACLE_JAR_DIR`'s expected cache path first, so
    /// `fetch_server_jar`'s own already-exists-and-hash-matches fast path still
    /// verifies it against Mojang's declared SHA-1 (Context).
    pub server_jar: Option<std::path::PathBuf>,
    /// Skip all network access. Requires `server_jar`. SHA-1 verification against
    /// Mojang's declared value and the live per-version Java-requirement lookup are
    /// both skipped (a warning is printed for each); `java_check::FALLBACK_MIN_JAVA_MAJOR`
    /// is used as the Java-version floor instead.
    pub offline: bool,
}

pub struct FetchOutcome {
    /// `datagen-output/<version>/generated/reports/` — `fetch_data::run_data_reports`'s
    /// own return path, exactly what `codegen`'s `reports_dir` argument should point
    /// at.
    pub reports_dir: std::path::PathBuf,
    /// SHA-1 (lowercase hex) of the jar actually used — feeds `codegen`'s
    /// `source_jar_sha1` provenance field regardless of whether it was downloaded,
    /// supplied via `--server-jar`, or hashed locally under `--offline`. `run` also
    /// persists this value to an `oracle/<version>/server.jar.sha1` sidecar text file
    /// (no trailing newline), since a later, separate `xtask codegen` process
    /// invocation cannot read this in-memory struct across the process boundary and
    /// must not re-hash the (possibly large) jar just to recover it.
    pub jar_sha1: String,
}

/// Orchestrates the full verb by calling `crate::fetch_data::fetch_server_jar` /
/// `crate::fetch_data::run_data_reports` for every ordinary (non-offline) case
/// (Context) — never re-resolving piston-meta or re-invoking `--reports` itself.
/// Concrete error cases and their exact actionable messages (Implementation steps
/// below give the precise wording for each): `offline` given without `server_jar`;
/// `server_jar` given but the path does not exist; any `fetch_data::FetchDataError`
/// the shared primitive itself produces (version not found, network failure, hash
/// mismatch, java not found, `--reports` failure), surfaced via its own `Display`
/// impl; local Java below the required major version (`check_java`, this
/// blueprint's own module, run against whichever `min_java_major` the online or
/// offline path produced).
pub fn run(args: &FetchArgs) -> Result<FetchOutcome, String>;
```

### `xtask/src/fixture_manifest.rs`

```rust
//! TEST-D47's fixture integrity manifest, restated concretely for this blueprint's
//! generated artifacts (and reusable by any later blueprint that generates other
//! fixture kinds TEST-D47 also covers — golden data, `rc-gametest` structures, worldgen
//! seed-corpus entries — none of which this blueprint produces).

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct FixtureManifest {
    pub protocol_version: u32,
    pub mc_version: String,
    /// One row per fixture, per TEST-D47's exact wording: "relative path, SHA-256 of
    /// the fixture's own bytes, the generator/tool version that produced it, and the
    /// source vanilla-jar hash it was derived from."
    pub entries: Vec<FixtureEntry>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct FixtureEntry {
    /// Relative to the manifest file's own directory.
    pub path: String,
    /// Lowercase hex, 64 characters.
    pub sha256: String,
    pub generator_tool_version: String,
    /// SHA-1 (lowercase hex) of the `server.jar` this fixture was derived from.
    pub source_jar_sha1: String,
}

/// One manifest-vs-disk discrepancy.
pub struct ManifestViolation {
    pub path: String,
    /// `"missing"` | `"hash_mismatch"`.
    pub kind: &'static str,
    pub message: String,
}

pub fn compute_sha256_hex(bytes: &[u8]) -> String;

/// Builds a manifest whose every entry's `sha256` is `compute_sha256_hex` of that
/// file's own bytes; `generator_tool_version`/`source_jar_sha1` are copied verbatim
/// onto every entry from the corresponding arguments.
pub fn build_manifest(
    protocol_version: u32,
    mc_version: &str,
    files: &[(String, Vec<u8>)],
    generator_tool_version: &str,
    source_jar_sha1: &str,
) -> FixtureManifest;

/// Reads the manifest JSON at `manifest_path`, and for every listed entry reads
/// `base_dir.join(&entry.path)`, recomputes its SHA-256, and compares. Returns one
/// `ManifestViolation` per entry whose file is missing or whose hash does not match;
/// an empty result means every listed fixture verified. Does not flag files present on
/// disk but absent from the manifest (out of this blueprint's stated scope — see
/// Constraints).
pub fn verify_manifest(
    manifest_path: &std::path::Path,
    base_dir: &std::path::Path,
) -> Vec<ManifestViolation>;
```

### `xtask/src/datagen/mod.rs`

```rust
pub mod codegen;
pub mod fetch;
pub mod java_check;
pub mod reports;
```

(No `version_manifest` module — that responsibility lives entirely in M0-B08's `crate::fetch_data`, reused directly by `fetch.rs`, Context.)

### `xtask/src/lib.rs` (modify — add two re-exports, alongside M0-B01's existing ones)

```rust
pub mod datagen;
pub mod fixture_manifest;
```

### `xtask/src/main.rs` (modify — extend `Command`, unchanged variants shown for context)

```rust
#[derive(clap::Subcommand, Debug, PartialEq)]
pub enum Command {
    /// cargo fmt --all -- --check
    FmtCheck,
    /// cargo clippy --workspace --all-targets -- -D warnings
    Lint,
    /// WS-D3 dependency-graph rule checker
    LintDeps,
    /// nextest (default features) + rusty-clanker-server monolithic + doctests
    Test,
    /// NET-D9: download (or accept via --server-jar) the pinned version's server.jar
    /// and run its --reports data generator locally, via M0-B08's shared
    /// `fetch_data` primitive (Context).
    FetchData {
        /// Minecraft version id, e.g. "26.2".
        version: String,
        #[arg(long)]
        server_jar: Option<std::path::PathBuf>,
        #[arg(long)]
        offline: bool,
    },
    /// NET-D9: read a prior FetchData run's cached reports (under `datagen-output/`,
    /// M0-B08's shared path) and emit generated Rust registry/block-state tables plus
    /// a TEST-D47 fixture manifest.
    Codegen {
        /// Minecraft version whose cached reports to read.
        #[arg(long, default_value = "26.2")]
        version: String,
        /// Output directory name suffix — protocol_version is an opaque, hand-bumped
        /// integer never derivable from --reports (docs/research/mc-26.2/00-source-
        /// overview.md's own note: "not derived from anything else... bumped by hand
        /// per release"), so it is a flag with NET-D1's current pin as its default,
        /// not something parsed out of the fetched data.
        #[arg(long, default_value_t = 776)]
        protocol_version: u32,
    },
    /// TEST-D47: recompute crates/registries/generated/v776/MANIFEST.json's hashes
    /// against the files on disk and fail on any mismatch.
    VerifyGenerated,
}
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** this blueprint's test changeset is exactly the five files below, plus `xtask/src/datagen/{mod.rs, java_check.rs, reports.rs, codegen.rs, fetch.rs}`, `xtask/src/fixture_manifest.rs`, and the `xtask/src/{lib.rs, main.rs}` edits — every function body from the Deliverables signatures replaced with `todo!()` (fields/derives/doc comments unchanged), plus the `Cargo.toml`/root-`Cargo.toml` edits. The implementation changeset (Implementation steps below) fills in real bodies only; it must not modify any of the five test files, and must not touch `crates/registries/generated/v776/*` (that directory's real content is a separate, manually-performed step per Constraints — not part of either changeset here). There is no `datagen_version_manifest.rs` test file — piston-meta manifest parsing lives entirely in M0-B08's `fetch_data.rs`, reused rather than duplicated (Context), so there is no second copy of that logic here for this blueprint to test.

### `xtask/tests/datagen_java_check.rs`

1. `parses_modern_single_number_version` — `parse_java_major("openjdk version \"25\" 2026-04-21\nOpenJDK Runtime Environment...\n") == Some(25)`.
2. `parses_dotted_version` — `parse_java_major("openjdk version \"25.0.1\" 2026-05-01\n...") == Some(25)`.
3. `parses_legacy_one_dot_scheme` — `parse_java_major("java version \"1.8.0_301\"\n...") == Some(8)`.
4. `returns_none_on_unparseable_input` — `parse_java_major("not a java version string") == None`.

### `xtask/tests/datagen_reports_parsing.rs`

1. `parses_registries_report_minimal` — `serde_json::from_str::<RegistriesReport>` on a literal matching Context's `registries.json` excerpt (2 entries under `minecraft:block`); assert `report["minecraft:block"].entries["minecraft:air"].protocol_id == 0` and `["minecraft:stone"].protocol_id == 1`.
2. `parses_blocks_report_and_finds_default_state` — `serde_json::from_str::<BlocksReport>` on a literal with two blocks: a single-state block (`air`, one state, `default: true`) and a multi-state block (3 states, the middle one flagged `default: true`, matching Context's `oak_door`-shaped excerpt); `find_default_state_id` returns `Some(0)` for the first, `Some(<the flagged state's id>)` for the second.
3. `find_default_state_id_returns_none_when_unflagged` — a `BlockReport` with two states, neither flagged `default: true`; `find_default_state_id` returns `None`.

### `xtask/tests/datagen_codegen.rs`

1. `generates_registries_module_sorted_by_protocol_id` — a synthetic `RegistriesReport` with one registry (`minecraft:block`) whose JSON-literal `entries` object lists `stone` (protocol_id 1) *before* `air` (protocol_id 0) in source-text order; call `generate`; in the returned `"registries.rs"` content, assert the byte offset of `RegistryEntryId(0)`'s line is less than `RegistryEntryId(1)`'s line (proves explicit sort-by-id, not incidental map/text order).
2. `sanitizes_slash_and_keyword_identifiers` — a `RegistriesReport` with two registries named `minecraft:worldgen/biome` and `minecraft:type`; assert `generate`'s `"registries.rs"` output contains `pub mod worldgen_biome {` and `pub mod type_ {` (the keyword-collision guard — see Deliverables' `sanitize_mod_name` algorithm, Implementation steps).
3. `output_is_independent_of_input_insertion_order` — build two logically-identical `(RegistriesReport, BlocksReport)` pairs containing the same 3 registry entries and 2 blocks, but constructed via `.insert()` calls in reverse order between variant A and variant B; assert `generate(&a.0, &a.1).files == generate(&b.0, &b.1).files` (byte-for-byte equal `Vec<(String,String)>`).
4. `block_states_module_reports_correct_counts_and_default_ids` — a synthetic `BlocksReport` with 2 blocks (one 1-state, one 3-state, each with exactly one `default: true`); assert `"block_states.rs"`'s content contains `pub const BLOCK_TYPE_COUNT: u32 = 2;`, `pub const BLOCK_STATE_COUNT: u32 = 4;`, and one `pub const <NAME>: BlockStateId = BlockStateId(<id>);` per block whose `<id>` matches that block's flagged-default state id.
5. `generated_files_compile_standalone` — write `generate(...)`'s two file contents (from a small synthetic report, not real Mojang data) to a fresh `tempfile`-free `std::env::temp_dir()` subdirectory; for each file, shell out to `rustc --edition 2024 --crate-type lib --out-dir <that dir> <file path>` (via `std::process::Command`, not `xshell` — no `xshell::Shell` dependency needed for a single one-shot compile check); assert exit status success for both. This is the jar-independent proof that `codegen`'s output shape is valid, self-contained (no external crate references), compiling Rust — the mechanical core of M0's roadmap Acceptance Criterion 3, exercised without needing a real jar.

### `xtask/tests/fixture_manifest.rs`

1. `sha256_matches_known_vector` — `compute_sha256_hex(b"") == "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"` (the well-known SHA-256 of the empty byte string).
2. `build_manifest_populates_every_field_per_entry` — `build_manifest(776, "26.2", &[("registries.rs".into(), b"content-a".to_vec()), ("block_states.rs".into(), b"content-b".to_vec())], "xtask-codegen/0.1.0", "deadbeef00000000000000000000000000000000")`; assert `.entries.len() == 2`; for each entry assert `.sha256 == compute_sha256_hex(<that file's own bytes>)` and `.generator_tool_version`/`.source_jar_sha1` equal the passed-in values.
3. `verify_manifest_passes_on_untampered_files` — write a manifest (via `build_manifest` + `serde_json::to_writer`) and its two referenced files into a fresh temp directory with matching content; `verify_manifest(...).is_empty()`.
4. `verify_manifest_detects_hash_mismatch` — same setup as test 3, then overwrite one referenced file's on-disk bytes afterward (simulating a hand-edit); assert `verify_manifest(...)` returns exactly one `ManifestViolation` with `kind == "hash_mismatch"` for that file's `path`.
5. `verify_manifest_detects_missing_file` — same setup as test 3, then delete one referenced file before calling `verify_manifest`; assert exactly one violation with `kind == "missing"` for that file's `path`.

### `xtask/tests/datagen_cli_parsing.rs`

```rust
use xtask::{Cli, Command};
use clap::Parser;
```

1. `parses_fetch_data_with_version_only` — `Cli::try_parse_from(["xtask", "fetch-data", "26.2"])` matches `Command::FetchData { version, server_jar: None, offline: false } if version == "26.2"`.
2. `parses_fetch_data_with_server_jar_flag` — `["xtask", "fetch-data", "26.2", "--server-jar", "C:/tmp/server.jar"]` → `server_jar == Some(PathBuf::from("C:/tmp/server.jar"))`.
3. `fetch_data_requires_version_argument` — `["xtask", "fetch-data"]` (no version) → `.is_err()`.
4. `parses_codegen_with_defaults` — `["xtask", "codegen"]` matches `Command::Codegen { version, protocol_version: 776 } if version == "26.2"`.
5. `parses_codegen_with_explicit_protocol_version` — `["xtask", "codegen", "--protocol-version", "777"]` → `protocol_version == 777`.
6. `parses_verify_generated` — `["xtask", "verify-generated"]` → `Command::VerifyGenerated`.

## Implementation steps

1. **`xtask/Cargo.toml` + root `Cargo.toml`.** Apply the two edits from Deliverables exactly (no `.gitignore` edit — M0-B08's). Observable: `cargo metadata` still succeeds; `cargo build -p xtask` still compiles (deps resolve).
2. **`xtask/src/datagen/java_check.rs`.** `parse_java_major`: find the first `"`, then the next `"` after it; the substring between them is the version string; split on `.`/`-`, take the first component, parse as `u32`; if that parses to `1`, take the *second* `.`-split component instead (legacy scheme) and parse that. `check_java(min_major)`: run `std::process::Command::new("java").arg("-version").output()`; on `Err` (not found on PATH) return `Err("java not found on PATH. Minecraft's data generator requires a local Java {min_major}+ runtime — install one (e.g. https://adoptium.net) and ensure `java -version` succeeds before retrying.")`; on success concatenate `stdout`+`stderr` as UTF-8 (lossy), call `parse_java_major`; `None` → actionable "could not parse java -version output" error including the raw captured text; `Some(major)` where `major < min_major` → actionable error naming both `major` and `min_major` plus the same adoptium.net pointer; `Some(major) if major >= min_major` → `Ok(major)`. Observable: unit-testable via `datagen_java_check.rs`'s pure-parsing tests without invoking a real `java` binary; `check_java` itself is exercised only by the manual verification procedure (Verification commands).
3. **`xtask/src/datagen/reports.rs`.** Pure `#[derive(Deserialize)]` structs, as shown. `find_default_state_id`: `block.states.iter().find(|s| s.default).map(|s| s.id)`. Observable: compiles; `datagen_reports_parsing.rs` passes.
4. **`xtask/src/datagen/codegen.rs` — `generate`.** Implement `generate_registries_rs`/`generate_block_states_rs`/`sanitize_mod_name`/`sanitize_const_name`/`strip_namespace`/`is_rust_keyword` exactly per the pseudocode below (this is the algorithm's full, binding specification — not left to implementer judgment, per the blueprint spec's "algorithms are specified precisely" requirement):

   ```rust
   fn strip_namespace(id: &str) -> &str {
       id.split_once(':').map(|(_, path)| path).unwrap_or(id)
   }

   fn sanitize_mod_name(path: &str) -> String {
       let mut s: String = path.chars()
           .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
           .collect();
       if s.chars().next().is_some_and(|c| c.is_ascii_digit()) { s.insert(0, '_'); }
       if is_rust_keyword(&s) { s.push('_'); }
       s
   }

   fn sanitize_const_name(path: &str) -> String {
       // SCREAMING_SNAKE_CASE output never collides with a Rust keyword (keywords are
       // always lowercase), so no keyword guard is needed here.
       let mut s: String = path.chars()
           .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' })
           .collect();
       if s.chars().next().is_some_and(|c| c.is_ascii_digit()) { s.insert(0, '_'); }
       s
   }

   fn is_rust_keyword(s: &str) -> bool {
       matches!(s, "as"|"break"|"const"|"continue"|"crate"|"else"|"enum"|"extern"|"false"
           |"fn"|"for"|"if"|"impl"|"in"|"let"|"loop"|"match"|"mod"|"move"|"mut"|"pub"|"ref"
           |"return"|"self"|"static"|"struct"|"super"|"trait"|"true"|"type"|"unsafe"|"use"
           |"where"|"while"|"async"|"await"|"dyn"|"abstract"|"become"|"box"|"do"|"final"
           |"macro"|"override"|"priv"|"typeof"|"unsized"|"virtual"|"yield"|"try")
   }
   ```

   `generate_registries_rs`: emit a fixed, timestamp-free header comment (crate-level `//!` doc naming NET-D9/NET-D10 and "generated by `xtask codegen` — do not edit by hand, re-run instead"), then `pub struct RegistryEntryId(pub u32)` with the exact derive list from Deliverables' illustrative excerpt (`Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash`), then for each `(registry_name, report)` in the input `BTreeMap` (already name-sorted): collect `(entry_name, protocol_id)` pairs, `sort_by_key` on `protocol_id`, emit `pub mod {sanitize_mod_name(strip_namespace(registry_name))} { use super::RegistryEntryId; ...; pub const {sanitize_const_name(strip_namespace(entry_name))}: RegistryEntryId = RegistryEntryId({protocol_id}); ...; pub const COUNT: u32 = {entries.len()}; }`. `generate_block_states_rs`: same header pattern, `pub struct BlockStateId(pub u32)`, `pub const BLOCK_TYPE_COUNT: u32 = {blocks.len()}`, `pub const BLOCK_STATE_COUNT: u32 = {blocks.values().map(|b| b.states.len()).sum()}`, then `pub mod default_state { use super::BlockStateId; ... }` with one `pub const {sanitize_const_name(strip_namespace(block_name))}: BlockStateId = BlockStateId({find_default_state_id(block).expect("every real block report entry has exactly one default state")});` per block, iterated in the input `BTreeMap`'s (name-sorted) order. Observable: `datagen_codegen.rs`'s five tests all pass.
5. **`xtask/src/datagen/codegen.rs` — `run`.** Read `reports_dir.join("registries.json")`/`.join("blocks.json")` via `std::fs::read_to_string`; on I/O error return `Err(format!("missing {file} under {reports_dir} — run `cargo xtask fetch-data {version}` first"))` naming the exact missing filename. Parse both via `serde_json::from_str`. Call `generate`. Create `out_dir` (`std::fs::create_dir_all`). Write `registries.rs`/`block_states.rs`. Build the manifest via `fixture_manifest::build_manifest(args.protocol_version, &args.mc_version, &files_as_bytes, codegen::CODEGEN_TOOL_VERSION, &args.source_jar_sha1)`, serialize with `serde_json::to_string_pretty`, write `MANIFEST.json`. Immediately call `fixture_manifest::verify_manifest(&out_dir.join("MANIFEST.json"), &out_dir)`; if non-empty, return `Err` describing the self-check failure (an internal-bug signal, not a user-facing data problem). Observable: exercised only by the manual verification procedure (no automated test — needs real reports content on disk to be meaningful beyond what `generate`'s own tests already cover).
6. **`xtask/src/datagen/fetch.rs` — `run`.** First, a private `fn repo_root() -> PathBuf` helper: `PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().expect("xtask always lives directly under the workspace root")` — mirrors `crate::fetch_data`'s own identical, independent resolution (M0-B08); both modules compute the same path this way rather than one calling into the other's private internals. Then, in order: (a) if `args.offline && args.server_jar.is_none()`, return `Err("--offline requires --server-jar <path> (nothing to run --reports against without either a download or a supplied jar)".into())`. (b) If `args.server_jar` is `Some(path)` and `!path.exists()`, return `Err(format!("--server-jar path '{}' does not exist. Either omit --server-jar to let fetch-data download it automatically, or supply the correct path to a legally-obtained server.jar for Minecraft {} (protocol 776 at time of writing).", path.display(), args.version))`. (c) `let root = repo_root();`. (d) Obtain a `fetch_data::FetchedJar`: if `args.offline`, read the supplied jar's bytes from disk, copy them to `root.join(crate::fetch_data::ORACLE_JAR_DIR).join(&args.version).join("server.jar")` (create dirs; skip the copy if the source path already is that path), compute `sha1_hex` via `sha1::{Sha1, Digest}::digest(&jar_bytes)` formatted as lowercase hex (the exact same byte-to-hex pattern `fixture_manifest::compute_sha256_hex`, step 7 below, uses for SHA-256 — no separate `hex` crate needed for either), and construct `fetch_data::FetchedJar { jar_path: <that cache path>, version_id: args.version.clone(), sha1: sha1_hex, min_java_major: java_check::FALLBACK_MIN_JAVA_MAJOR }` directly; else (online), if `args.server_jar` is `Some(path)`, copy its bytes to that same cache path first (create dirs; skip if already there), then call `crate::fetch_data::fetch_server_jar(&args.version, &root)`, mapping any `Err(e)` to `Err(e.to_string())`. (e) `let fetched = <the FetchedJar from (d)>; java_check::check_java(fetched.min_java_major)?;` (f) `let reports_dir = crate::fetch_data::run_data_reports(&fetched, &root).map_err(|e| e.to_string())?;` (g) Write `fetched.sha1` as a plain-text sidecar file, `root.join(crate::fetch_data::ORACLE_JAR_DIR).join(&args.version).join("server.jar.sha1")` (no trailing newline) — this is the persistence mechanism that lets a *separate, later* `xtask codegen` process invocation (step 8 below) recover the jar's hash without re-reading or re-hashing the (possibly large) jar file, since `FetchOutcome` itself lives only in `fetch-data`'s own process memory and cannot cross a process boundary. (h) Return `FetchOutcome { reports_dir, jar_sha1: fetched.sha1 }`.
7. **`xtask/src/fixture_manifest.rs`.** `compute_sha256_hex`: `use sha2::{Sha256, Digest}; let mut hasher = Sha256::new(); hasher.update(bytes); hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()`. `build_manifest`: map `files` into `FixtureEntry { path: name.clone(), sha256: compute_sha256_hex(bytes), generator_tool_version: generator_tool_version.to_string(), source_jar_sha1: source_jar_sha1.to_string() }`. `verify_manifest`: read+parse the manifest JSON (I/O/parse error → a single violation with `kind: "missing"`, `path: manifest_path.display().to_string()`, describing the read/parse failure); for each entry, `std::fs::read(base_dir.join(&entry.path))` — `Err` → push `ManifestViolation{path: entry.path.clone(), kind: "missing", message: format!("{} listed in the manifest but not found on disk", entry.path)}`; `Ok(bytes)` → compute hash, if `!= entry.sha256` push `ManifestViolation{path: entry.path.clone(), kind: "hash_mismatch", message: format!("{}: manifest says {}, disk has {}", entry.path, entry.sha256, actual)}`. Observable: `fixture_manifest.rs`'s five tests all pass.
8. **`xtask/src/main.rs` dispatch.** `FetchData{..}` → call `fetch::run`, print outcome or error, map to `ExitCode`. `Codegen{..}` → resolve `root` (the same private `repo_root()` pattern step 6 uses), build `reports_dir = root.join(crate::fetch_data::DATAGEN_OUTPUT_DIR).join(&version).join("generated").join("reports")`, `out_dir = root.join("crates/registries/generated/v{protocol_version}")`, `source_jar_sha1` = the contents of the sidecar file `root.join(crate::fetch_data::ORACLE_JAR_DIR).join(&version).join("server.jar.sha1")` (step 6g) — read via `std::fs::read_to_string`, `Err` → actionable "run `cargo xtask fetch-data {version}` first" (same message pattern as `codegen::run`'s own missing-report-file case); call `codegen::run`. `VerifyGenerated` → call `fixture_manifest::verify_manifest` against the hardcoded `crates/registries/generated/v776/MANIFEST.json` (path relative to that same `repo_root()`), print violations or success, map to `ExitCode`.
9. **Run the automated test suite.** `cargo nextest run -p xtask` — all five new test files' cases pass, using only synthetic data; M0-B01's existing `xtask` tests (`lint_deps_rules.rs`, `cli_parsing.rs`) remain green, untouched.
10. **Full-workspace gates.** `cargo run -p xtask -- fmt-check`, `-- lint`, `-- lint-deps`, `-- test` all still exit 0.
11. **(Manual, requires a legal jar — not part of this blueprint's own CI-checkable Done state.)** Whoever has legal access to a Minecraft 26.2 `server.jar` (or network access for `fetch-data` to obtain one) runs `cargo xtask fetch-data 26.2` then `cargo xtask codegen`, confirms `crates/registries/generated/v776/{registries.rs, block_states.rs, MANIFEST.json}` now exist, runs `cargo xtask verify-generated` (expects exit 0), and confirms both `.rs` files compile via `rustc --edition 2024 --crate-type lib` (or, if also choosing to wire `crates/registries/src/lib.rs` to include them — optional, left to M1-B05's wiring — via `cargo build -p rc-registries`). The resulting three files are then committed in their own changeset (NET-D10/ASSET-D15: generated Rust + manifest are committable; the jar and raw reports JSON never are, and stay under `.gitignore`d `/oracle/`/`/datagen-output/`, M0-B08). This step satisfies M0's roadmap Acceptance Criterion 3.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** The five `xtask/tests/datagen_*.rs`/`fixture_manifest.rs` files are committed first, with every `xtask/src/{datagen/*.rs, fixture_manifest.rs}` function body they call stubbed `todo!()`. The implementation changeset (Implementation steps 1–10) fills in real bodies only; it must not edit any of the five test files, must not weaken or remove any of their cases, and must not touch `crates/registries/generated/v776/*` (step 11 is a separate, manually-performed step outside both changesets, gated on legal jar access — never bundled into either automated changeset).

(b) **No new external dependencies beyond the pinned set, with exactly one named exception.** `sha1` is already in `[workspace.dependencies]` (12-workspace-structure.md, NET-D6) and already added to `xtask/Cargo.toml` by M0-B08 (for `fetch_data.rs`). `sha2` is this blueprint's own cited addition at TEST-D47's exact required algorithm, version-matched to `sha1`'s already-pinned `0.11.0` — see Context for the full citation. This blueprint does **not** add `reqwest` itself — all network access goes through M0-B08's already-present `fetch_data` module, reused, never re-added (Context, Prerequisites). Do not add `anyhow`, `regex`, `cargo_metadata`, `tokio`, or any other crate not named here.

(c) **No Mojang or third-party reimplementation code.** Every JSON shape this blueprint parses is derived from `docs/research/mc-26.2/{00-source-overview.md, 07-blocks-blockstates.md}` (themselves produced under the ASSET-D18/D30 research-role process) and from Mojang's own publicly documented `piston-meta` launcher API (ASSET-D18(b)/(d)) — no decompiled source, no third-party reimplementation's code, is consulted or copied while writing any file this blueprint creates.

(d) **The generated `.rs` files' identifier-sanitization and codegen algorithm (step 4's pseudocode) is binding, not a suggestion** — an implementer must not substitute a different sanitization scheme, sort key, or output layout, since `datagen_codegen.rs`'s acceptance tests assert on this exact algorithm's observable output (module names, ordering, keyword-escaping).

(e) **Never embed a wall-clock timestamp, hostname, or any other run-varying value in generated output.** This is required for TEST-D47's manifest-hash scheme to be meaningful (Context's "Determinism" subsection) — no field for it exists in any Deliverables struct, and none may be added.

(f) **Packet codegen and `crates/protocol/` stay out of scope.** This blueprint does not author packet field-layout spec files (`crates/protocol/spec/*`), emits nothing under `crates/protocol/generated/` (WS-D13 reserves that directory for packet code), does not implement `#[derive(RcPacket)]` in `rc-protocol-macros`, and does not modify `crates/protocol/` or `crates/registries/src/lib.rs`/`Cargo.toml` — packet codegen is reserved for the M1 protocol-bootstrap blueprint per Context's "Resolved scope boundary" note, and the module-tree wiring of this blueprint's own output is M1-B05's.

(g) **No `unsafe` code.** Every function in this blueprint's deliverables is implementable in 100% safe Rust.

## Verification commands

Automated, run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43) — no jar, no network, no local Java required:

```
cargo build -p xtask --all-features
cargo nextest run -p xtask
cargo run -p xtask -- fmt-check
cargo run -p xtask -- lint
cargo run -p xtask -- lint-deps
cargo run -p xtask -- test
```

Expected: every command exits 0. `cargo nextest run -p xtask` includes all five of this blueprint's new test files — `datagen_java_check.rs` (4 cases), `datagen_reports_parsing.rs` (3), `datagen_codegen.rs` (5), `fixture_manifest.rs` (5, note: one — `sha256_matches_known_vector` — has no filesystem dependency), `datagen_cli_parsing.rs` (6) — 23 cases in total — alongside M0-B01's existing `xtask` suite and M0-B08's own `xtask` test suite (Prerequisites), all green.

Manual, requires a locally supplied or network-fetchable legal Minecraft 26.2 `server.jar` and a local Java 25+ runtime (NET-D9: never run by CI in this blueprint's own gate) — this is M0's roadmap Acceptance Criterion 3:

```
cargo xtask fetch-data 26.2
cargo xtask codegen
cargo xtask verify-generated
rustc --edition 2024 --crate-type lib --out-dir <tmp> crates/registries/generated/v776/registries.rs
rustc --edition 2024 --crate-type lib --out-dir <tmp> crates/registries/generated/v776/block_states.rs
```

Expected: every command exits 0; `crates/registries/generated/v776/{registries.rs, block_states.rs, MANIFEST.json}` exist and, once committed, are what all future ordinary CI runs build against — with no further jar or network dependency from that point on. CI (`.github/workflows/ci.yml`, M0-B01) green on both `ubuntu-24.04` and `windows-2025` legs for the automated portion above is this blueprint's own authoritative done-signal (TEST-D50); the manual portion's completion is confirmed once, by whoever performs it, and its result (the three committed files) is what subsequent CI runs then verify automatically forever after.
