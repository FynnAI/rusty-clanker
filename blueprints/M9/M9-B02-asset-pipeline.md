# M9-B02 — Local Asset Pipeline (`rc-assets`)

| Field | Content |
|---|---|
| ID | M9-B02 |
| Milestone | M9 — Client Bootstrap |
| Prerequisites | M0-B01 (workspace scaffold — `rc-assets` exists as an empty-shell crate with `rc-core`/`rc-registries` path dependencies already wired; this blueprint is the first to give it real content) |
| Implements | ASSET-D11 (`.minecraft` layout, discovery), ASSET-D12 (verified jar-vs-index resource split), ASSET-D13 (local-only acquisition, fail-fast, no CDN fallback), ASSET-D14 (SHA-1 integrity, no repair), ASSET-D18/D19 (jar-as-pure-data-container legal framing, public-documentation sourcing for the blockstate/model JSON schemas), ASSET-D24 (release content-audit scanner seam); CLIENT-D11 (`.mcmeta` animation *parsing* only — playback is a later blueprint's), CLIENT-D14 (blockstate/model JSON *raw parsing* only — interpretation/baking is M9-B05's); WS-D1–D3/D7 (crate placement, dependency edges, external-version pins); TEST-D45–D47 (test-first changeset boundary, fixture custody, integrity manifest) |
| Crates touched | `rc-assets` (`crates/assets/`) — full implementation; `xtask` (`xtask/`) — adds one new verb, `content-audit` |
| Estimated scope | L (upper bound of this tier — the domain is broad, but stays one coherent, mergeable unit: one crate, one cache, one facade type, no cross-crate coordination) |

## Goal & Done definition

Give `rc-assets` its first real content: locate and validate the player's local `.minecraft` installation against the pinned protocol target (26.2, NET-D1); read the version's client jar as a plain zip data container; parse the asset index and resolve hash-addressed objects with SHA-1 verification; resolve an ordered resource-pack stack (base game + zero or more override packs) with correct highest-priority-wins path resolution; decode PNG textures and parse their `.mcmeta` animation sidecars; parse (never bundled, never distributed) blockstate and model JSON into raw, validated-but-uninterpreted Rust types that a later blueprint (M9-B05) bakes into renderable face lists; cache everything keyed to the resolved pack stack's identity, invalidating on any pack-set change; and give `08-assets-auth-legal.md`'s ASSET-D24 release content-audit rule a concrete, mechanically-checkable seam (a new `xtask content-audit` verb). Every acceptance test in this blueprint runs against hand-authored, from-scratch fixture files — never a real Mojang asset, never a real vanilla jar — so this blueprint's own CI tier needs no oracle bootstrap (TEST-D41) at all.

Done when:

- [ ] `cargo build -p rc-assets` and `cargo build -p xtask` succeed with zero warnings.
- [ ] Every acceptance test in this blueprint's test-authoring changeset (Acceptance tests section) passes under `cargo nextest run -p rc-assets` and `cargo nextest run -p xtask -- content_audit`.
- [ ] `cargo clippy -p rc-assets -p xtask --all-targets -- -D warnings` is clean.
- [ ] No file under `crates/assets/src/` or `xtask/src/content_audit.rs` contains `include_bytes!`, `include_str!`, or any `build.rs`-driven asset embedding (grep-verifiable — see Constraints).
- [ ] `cargo run -p xtask -- content-audit <dir>` exits non-zero against a fixture directory deliberately seeded with a forbidden file, and exits zero against a clean one (Acceptance tests, `content_audit` module).
- [ ] CI tier: Tier 1 (TEST-D37) — this blueprint adds no nightly/oracle-dependent content; the separate, non-gating, dev-machine-only verification pass against a real, legally-owned 26.2 installation (see Context's closing note) is documented but never required for this blueprint's own CI to be green.

## Context (self-contained)

### Scope boundary — what this blueprint does NOT do

Stated up front because several adjacent, easily-confused responsibilities exist elsewhere:

- **Interpretation/baking is M9-B05's**, not this blueprint's. `rc-assets` (this crate) produces raw, JSON-shaped parsed types (`RawBlockstate`, `RawModel`, decoded pixel buffers). Resolving a blockstate+property combination into a flattened face list, following the model `parent` inheritance chain, computing AO/tint/cullface at mesh-build time (CLIENT-D8/D9/D14) — none of that lives here.
- **Animation *playback* is a later blueprint's** (per-tick frame advance, GPU upload of the animation table, CLIENT-D11's `binding_array`/uniform-array mechanism). This blueprint only *parses* the `.mcmeta` animation JSON into a plain data struct.
- **Sound is out of M9's scope entirely.** M9's own boundary (per the milestone's Scope) excludes sound/entities/UI/chat — `sounds.json` parsing and the `kira` audio engine (CLIENT-D24) land in M10. `rc-assets`'s asset-index reader (below) can fetch *any* hash-addressed object's raw bytes generically, so a later blueprint gets sound support "for free" by calling the same `load_index_object` function this blueprint ships — but this blueprint parses no sound-specific structure.
- **Lang files are out of M9's scope**, for the same reason (M9 renders no GUI/text/chat yet, per CLIENT-D17's font pipeline being M10 work). Same story as sound: the generic asset-index object reader already covers the byte-fetch half; nothing sound- or lang-specific is added here.
- **Item model definitions** (`assets/<namespace>/items/*.json`, CLIENT-D16's post-1.21.2 dispatch format) are **not** parsed by this blueprint. The task scope this blueprint was derived from names blockstate/model JSON explicitly and does not name the item-model-definition format; parsing it is left to whichever blueprint first needs item rendering (M9-B05 or later) rather than guessed here.
- **`rc-registries` is not consumed by this blueprint's own code**, even though `rc-assets`'s `Cargo.toml` already carries a path edge to it (fixed by `12-workspace-structure.md`'s dependency graph, scaffolded by M0-B01). Every type this blueprint parses is keyed by string `ResourceLocation` (namespace:path), never by a numeric block-state ID — connecting a blockstate's `variants` keys to `rc-registries`' block-state ID space is explicitly M9-B05's job (CLIENT-D14: "cached by blockstate ID (from `rc-registries`)" describes the *baked* cache, not this blueprint's raw one).
- **Profile/identity data (ASSET-D7) is `rc-auth`'s**, not this crate's — named in this blueprint's originating task range only because that task cited ASSET-D7–D14 as a block; ASSET-D7 (Microsoft profile retrieval) has no local-asset-pipeline content and is not implemented here.

### `.minecraft` installation layout and discovery (ASSET-D11)

Standard per-OS locations, restated verbatim from `08-assets-auth-legal.md`'s ASSET-D11 (these are the official Mojang launcher's own conventions, unchanged for over a decade):

| OS | Path |
|---|---|
| Windows | `%APPDATA%\.minecraft` |
| macOS | `~/Library/Application Support/minecraft` |
| Linux | `~/.minecraft` |

`TEST-D34` runs no macOS CI leg (Windows/Linux-only target platforms) — the macOS branch below is implemented and unit-tested as a pure path-computation function, but never exercised end-to-end by a live macOS runner; this is a documented, accepted gap, not an oversight.

Relevant subtree (ASSET-D11's tree, restated):

```
.minecraft/
├── versions/
│   └── 26.2/                    NET-D1's pinned protocol target (protocol 776)
│       ├── 26.2.jar              client jar — pure zip data container (see below)
│       └── 26.2.json             version manifest
├── assets/
│   ├── indexes/
│   │   └── <assetIndexId>.json   asset index (id "32" for the live 26.2 index, ASSET-D12)
│   └── objects/
│       └── <hash[0:2]>/<hash>    hash-addressed objects
├── resourcepacks/                custom resource packs: a subdirectory per pack, or a `.zip` file
│                                  directly inside it (standard `.minecraft` layout since 1.6,
│                                  confirmed via minecraft.wiki's Resource_pack article — not
│                                  itself drawn into ASSET-D11's tree diagram, but the same
│                                  "standard per-OS launcher convention" category that tree covers)
└── launcher_accounts.json        NEVER read by Rusty Clanker (ASSET-D10) — irrelevant to this crate
```

### Verified resource split (ASSET-D12) — copied verbatim, this crate's load-bearing routing table

| Resource category | Resolved from | Present in live 26.2 asset index? |
|---|---|---|
| Block/item models | client jar, `assets/minecraft/models/{block,item}/` | absent |
| Blockstates | client jar, `assets/minecraft/blockstates/` | absent |
| Block/item/entity/GUI textures, title panorama | client jar, `assets/minecraft/textures/` | absent |
| `sounds.json` manifest | client jar, `assets/minecraft/sounds.json` | jar-resident (not parsed by this blueprint — see Scope boundary) |
| Sound files (`.ogg`) | asset index → `assets/objects/` | present (out of M9 scope) |
| Language files | asset index → `assets/objects/` | present (out of M9 scope) |
| Unicode font glyph sheets | asset index → `assets/objects/` | present (out of M9 scope) |
| Launcher/app icons, bundled resource-pack archives | asset index → `assets/objects/` | present, gameplay-irrelevant |

Every path this blueprint's own scope touches (blockstates, models, textures) therefore resolves through the **resource-pack stack** (below), never through the asset index — the asset index only ever matters for this blueprint's generic `load_index_object` escape hatch, used by later, out-of-M9-scope blueprints.

### Pinned-version match rule and local-only acquisition (ASSET-D13/D14)

`discover()` requires `versions/26.2/26.2.jar` **and** `versions/26.2/26.2.json` to both exist under the located `.minecraft` root. If either is missing, or `26.2.json`'s own `"id"` field does not read exactly `"26.2"`, discovery fails closed with `DiscoveryError::MissingPinnedVersion` carrying a diagnostic naming the missing version and instructing the player to launch `26.2` once via the official Minecraft Launcher — **matching ASSET-D13's exact policy: no download, no repair, no substitution, ever, by any code in this crate.** The same fail-closed, no-repair rule applies to every asset-index object this crate resolves (ASSET-D14): a SHA-1 mismatch is treated identically to "file missing" — a hard `ObjectResolveError`, never a silent re-fetch, since this crate makes zero network requests of any kind, period (`rc-assets` has no HTTP client dependency at all — the CDN-fetch prohibition is enforced structurally, not just by convention, since the crate literally cannot make an HTTP request without first adding a dependency this blueprint deliberately does not add).

### `client.jar` as a pure data container (ASSET-D18/D19 legal framing)

The client jar is opened exclusively through the `zip` crate (`8.6.0`, pinned, already annotated for `rc-assets` in `12-workspace-structure.md`'s dependency table) as a plain zip archive. **No `.class` entry inside it is ever read, loaded, or executed; no JVM is started; no Mojang code runs at any point.** This crate's only interaction with the jar's bytes is reading specific `assets/minecraft/...`-prefixed entries as opaque byte arrays and handing them to this crate's own from-scratch JSON/PNG parsers — the identical "read the file, never run the code" boundary the project already applies to `server.jar`'s `--reports` step (NET-D9), except stricter here since not even a subprocess is spawned.

### Asset index and version manifest schemas (field-by-field)

**Version manifest** (`versions/26.2/26.2.json`) — restating ASSET-D11's named fields as a concrete schema (extra fields present in a real manifest, e.g. `minimumLauncherVersion`, `arguments`, `libraries`, are silently ignored — this crate does not use `#[serde(deny_unknown_fields)]` anywhere, since it never needs to be a complete manifest parser, only a field-extractor):

| JSON path | Rust field | Type | Notes |
|---|---|---|---|
| `id` | `id` | `String` | must equal `"26.2"` |
| `assetIndex.id` | `asset_index.id` | `String` | live value `"32"` for 26.2 (ASSET-D12) — never hardcoded, always read live from this field |
| `assetIndex.url` | `asset_index.url` | `String` | **never fetched** (ASSET-D13) — parsed only so the field exists for completeness/diagnostics |
| `assetIndex.sha1` | `asset_index.sha1` | `String` | compared against a live-computed hash of the local `assets/indexes/<id>.json` file (ASSET-D14) |
| `assetIndex.size` | `asset_index.size` | `u64` | — |
| `assetIndex.totalSize` | `asset_index.total_size` | `u64` | — |
| `downloads.client.sha1` | `downloads_client_sha1` | `String` | compared against a live-computed hash of `versions/26.2/26.2.jar` (ASSET-D14, extended to the jar itself) |
| `downloads.client.size` | `downloads_client_size` | `u64` | — |

**Asset index** (`assets/indexes/<id>.json`) — the real, wiki.vg/minecraft.wiki-documented schema, restated field-by-field:

```json
{
  "objects": {
    "<logical/path/inside/assets/minecraft/...>": { "hash": "<40-char lowercase hex SHA-1>", "size": <bytes:u64> }
  }
}
```

One flat `objects` map; every key is a logical path (e.g. `"minecraft/sounds/random/click.ogg"`); every value carries the object's SHA-1 (used both as the integrity check, ASSET-D14, and as the on-disk address: `assets/objects/<hash[0:2]>/<hash>`) and its byte size. No other top-level key is read by this crate (the legacy pre-1.7 `"map_to_resources"` boolean is out of scope — NET-D2's single-pinned-version discipline means this crate never needs to support any format predating 26.2).

### Resource-pack stack: schema and stacking order

**`pack.mcmeta`** (one per pack, at that pack's root — for the base game, this crate reads it from the **client jar's own root**, i.e. `pack.mcmeta` sibling to `assets/` inside the jar, not `assets/minecraft/pack.mcmeta`):

| Field | Type | Notes |
|---|---|---|
| `pack.pack_format` | `i32` | required |
| `pack.description` | `String` \| text-component object | this crate flattens either shape to a plain `String` via a small recursive "concatenate every `text` field, ignore styling/color/click-events" extractor — rich formatting is out of scope (chat/text rendering is M10's, CLIENT-D17) |
| `pack.supported_formats` | `i32` \| `{"min_inclusive": i32, "max_inclusive": i32}` \| absent | optional; single-int form means `{min_inclusive: N, max_inclusive: N}` |
| `overlays.entries[]` | array of `{"formats": ..., "directory": string}` | **parsed and preserved verbatim, never applied** — see the resolved simplification below |

> **Resolved simplification (overlays):** `pack.mcmeta`'s `overlays` mechanism lets one pack ship multiple parallel `assets/`-shaped directories, selected by which `pack_format` range the running game falls into. Since this project pins exactly one version (NET-D2), at most one overlay entry could ever be relevant, and this blueprint does not implement overlay *selection* — `PackMeta.overlays` retains the raw parsed list so a future blueprint can add selection once needed, but `ResourceStack` resolution (below) reads only each pack's base `assets/` tree. This is a deliberate, bounded M9 simplification, not an oversight; reconciliation step: if 26.2 client resource resolution requires a non-default overlay directory, a follow-up blueprint adds `fn select_overlay(&PackMeta, running_format: i32) -> Option<&str>` and wires it into `build_stack`.

**Pinned-format compatibility is resolved dynamically, never hardcoded.** This crate never bakes in a specific "26.2's pack format is N" constant (26.2 has no publicly documented pack-format number to source, since it postdates every real released version at the time this blueprint was written). Instead, `build_stack` (Deliverables' `resourcepack.rs`) reads the client jar's own root `pack.mcmeta` internally, as one of its first steps, and checks every *other* enabled pack's `supported_formats`/`pack_format` against that live-read value — this reference read is not exposed as a separate public method, since nothing outside `build_stack` needs it. A pack whose format is outside range is not excluded from the stack — it is retained with a `PackMeta::format_compatible: bool` flag, so a UI layer (out of this crate's scope) can warn without this crate making a policy call about whether to still load it. The `Vanilla` entry's own `ResolvedPack.meta` stays `None` (Deliverables) — it is the reference the flag is computed against, not itself flagged.

**Stacking order (this blueprint's own resolved design, not a vanilla-launcher-file-format question):** `rc-assets` does **not** parse the vanilla launcher's `options.txt` at all — resource-pack selection and priority order is supplied by the *caller* as an explicit, already-ordered `&[PackRef]` (a config concern owned elsewhere, out of this blueprint's scope). This sidesteps a genuinely ambiguous, undocumented question (which end of `options.txt`'s `resourcePacks` array is highest-priority) entirely, while still faithfully reproducing vanilla's actual *stacking mechanic*, confirmed via minecraft.wiki's Resource_pack article: *"the bottom-most pack loads first, then each pack above it replaces or merges loaded assets with ones it contains."* This crate's `build_stack` reproduces exactly that: given a caller-supplied list `[pack_a, pack_b, pack_c]`, the resolved stack is `[Vanilla, pack_a, pack_b, pack_c]` — index 0 (`Vanilla`, the base game assets from the client jar) is always present, always lowest priority, always first-loaded; each subsequent entry overrides/merges on top of everything before it; the **last** entry in the caller-supplied list wins any direct path conflict.

### PNG texture decoding and `.mcmeta` animation sidecars (CLIENT-D11 parsing half)

PNG decode uses the `image` crate (`0.25.10`, pinned, `12`'s table annotates it "rc-assets texture decode") — decoded to a flat top-to-bottom row-major RGBA8 buffer (the crate's own default output layout for `DynamicImage::to_rgba8()`).

An animated texture's sidecar lives at `<texture-path>.png.mcmeta` — a **separate file path**, resolved independently through the exact same resource-pack-stack lookup as the `.png` itself (not specially tied to whichever pack supplied the `.png` — if an overriding pack ships a texture but no matching `.mcmeta`, resolution falls through the stack for the `.mcmeta` path on its own terms, which may find a lower-priority pack's sidecar or none at all; this is the literal consequence of "every path resolves independently," not a special case this crate adds).

`.mcmeta` animation schema (confirmed via minecraft.wiki, current and structurally unchanged for many versions):

```json
{
  "animation": {
    "interpolate": false,
    "width": 16,
    "height": 16,
    "frametime": 1,
    "frames": [0, 1, { "index": 2, "time": 3 }]
  }
}
```

| Field | Type | Default | Notes |
|---|---|---|---|
| `interpolate` | `bool` | `false` | playback concern, parsed and passed through only |
| `width` | `u32` | omitted → derived from image dimensions (this crate leaves it `None` when absent; deriving the effective tile size from the decoded image is the atlas-builder's job, not this crate's) |
| `height` | `u32` | same as `width` |
| `frametime` | `u32` | `1` | ticks per frame unless a frame entry overrides it |
| `frames` | array of (`u32` \| `{index: u32, time: Option<u32>}`) | omitted → empty, meaning "play every frame in order" (interpreted by the baking/playback stage, not here) | mixed plain-int and object entries are valid in the same array |

Any top-level `pack.mcmeta` section this crate does not name above (`villager`, `texture{blur,clamp}` and similar) is silently ignored — `McmetaFile` only ever looks at the `animation` key.

### Blockstate JSON schema (field-by-field, raw parse only)

Source: minecraft.wiki's Blockstates definition/format article (ASSET-D18(b) allowed source) — this format has been structurally stable for a very long time and is independently, extensively documented; no Mojang source was consulted.

```json
{
  "variants": {
    "facing=north,open=false": { "model": "minecraft:block/oak_door_bottom", "y": 270, "uvlock": true },
    "variant_key_2": [
      { "model": "...", "weight": 1 },
      { "model": "...", "weight": 3 }
    ]
  }
}
```
```json
{
  "multipart": [
    { "apply": { "model": "minecraft:block/oak_fence_post" } },
    { "when": { "north": "true" }, "apply": { "model": "minecraft:block/oak_fence_side" } },
    { "when": { "OR": [ { "north": "true" }, { "south": "true" } ] }, "apply": { "model": "..." } }
  ]
}
```

| Element | Field | Type | Default / notes |
|---|---|---|---|
| Blockstate file, top level | `variants` | `Option<HashMap<String, VariantValue>>` | mutually exclusive with `multipart` — enforced by `validate_blockstate`, not the type itself |
| Blockstate file, top level | `multipart` | `Option<Vec<MultipartCase>>` | — |
| Variant key string | — | `String` | e.g. `"facing=north,open=false"`, or the empty string `""` for a property-less block; not decomposed into a property map by this crate — that decomposition (against `rc-registries`' known property set) is M9-B05's |
| `VariantValue` | single model | `ModelRef` | untagged union with the array form |
| `VariantValue` | weighted array | `Vec<ModelRef>` | random selection math is M9-B05's |
| `ModelRef.model` | `model` | `String` | resource-location string, parsed to `ResourceLocation` by `validate_model_ref`, not by the type itself |
| `ModelRef.x` / `.y` / `.z` | `x`,`y`,`z` | `i32`, default `0` | expected ∈ {0,90,180,270}; flagged, not rejected, if outside that set |
| `ModelRef.uvlock` | `uvlock` | `bool`, default `false` | — |
| `ModelRef.weight` | `weight` | `u32`, default `1` | — |
| `MultipartCase.when` | `when` | `Option<WhenClause>` | absent ⇒ always applies |
| `MultipartCase.apply` | `apply` | `VariantValue` | same shape as a variant's value |
| `WhenClause` (flat form) | — | `HashMap<String, String>` | implicit AND across every key; a value may itself contain `\|`-separated alternatives (e.g. `"north\|south"`) meaning OR *within that one property* |
| `WhenClause::Or` | `OR` | `Vec<PropertyMap>` | top-level OR across the listed flat maps |
| `WhenClause::And` | `AND` | `Vec<PropertyMap>` | top-level AND across the listed flat maps (rare in practice, still valid) |

### Model JSON schema (field-by-field, raw parse only)

Source: minecraft.wiki's Model article (ASSET-D18(b)) — cross-checked live during this blueprint's own derivation.

| Top-level field | Rust field | Type | Default / notes |
|---|---|---|---|
| `parent` | `parent` | `Option<String>` | resource-location string, or a builtin sentinel (`"builtin/generated"`, `"builtin/entity"`, `"builtin/missing"`) — recognized by `BuiltinParent::try_from_str`, resolution of the inheritance chain itself is M9-B05's |
| `ambientocclusion` | `ambient_occlusion` | `Option<bool>` | `None` = inherit from parent chain; effective default at the chain's root is `true` |
| `textures` | `textures` | `Option<HashMap<String, String>>` | variable name → `"#other_variable"` or `"namespace:path"`; the reserved `"particle"` key names the breaking-particle texture |
| `elements` | `elements` | `Option<Vec<RawElement>>` | absent when `parent` supplies them (inherited, not re-specified) |
| `display` | `display` | `Option<HashMap<String, RawDisplayTransform>>` | keys: `thirdperson_righthand`, `thirdperson_lefthand`, `firstperson_righthand`, `firstperson_lefthand`, `gui`, `head`, `ground`, `fixed`, `on_shelf` — not validated against this fixed key set by the parser (an unrecognized key round-trips harmlessly; M9-B05 looks up only the keys it needs) |
| `gui_light` | `gui_light` | `Option<String>` | expected `"front"` \| `"side"`, default `"side"`; flagged (not rejected) if neither |

`RawDisplayTransform`:

| Field | Type | Default |
|---|---|---|
| `rotation` | `[f32; 3]` | `[0,0,0]` |
| `translation` | `[f32; 3]` | `[0,0,0]` (expected clamp range ±80 — clamping is M9-B05's) |
| `scale` | `[f32; 3]` | `[1,1,1]` (expected cap 4 — capping is M9-B05's) |

`RawElement`:

| Field | Type | Default / range |
|---|---|---|
| `from` | `[f32; 3]` | required; expected range −16..=32 |
| `to` | `[f32; 3]` | required; expected range −16..=32 |
| `rotation` | `Option<RawRotation>` | absent = no rotation |
| `shade` | `bool` | default `true` |
| `light_emission` | `u8` | default `0`; expected range 0..=15 |
| `faces` | `HashMap<Direction, RawFace>` | keys ∈ `{down, up, north, south, west, east}`; a direction absent from the map means that face does not render |

`RawRotation`:

| Field | Type | Default / range |
|---|---|---|
| `origin` | `[f32; 3]` | required |
| `axis` | `Axis` (`x`\|`y`\|`z`) | required |
| `angle` | `f32` | required; expected discrete set `{-45,-22.5,0,22.5,45}` (this crate applies the same discrete-set expectation uniformly to both block and item models, since sources disagree on whether items alone are restricted to this set — flagged as `> **Resolved (moderate confidence):**` below) |
| `rescale` | `bool` | default `false` |

> **Resolved (moderate confidence):** public sources disagree on whether the discrete `{-45,-22.5,0,22.5,45}` angle restriction applies to item-model elements only or to every model's elements uniformly. This blueprint applies the discrete-set check uniformly (both cases) as a **non-fatal validation flag** (`ValidationIssue::UnusualRotationAngle`), never a parse error — an out-of-set angle still parses and round-trips correctly, it is only flagged for a human/CI-log to notice. Reconciliation step: cross-check against a live 26.2 client's own shipped models during implementation; if blocks genuinely allow continuous angles, narrow the flag to item models only in a follow-up changeset (a one-line change, isolated to `validate_model`).

`RawFace`:

| Field | Type | Default |
|---|---|---|
| `uv` | `Option<[f32; 4]>` | `[x1,y1,x2,y2]`, 0..=16; absent = auto-generate from element position (M9-B05's job) |
| `texture` | `String` | required; a `"#variable"` reference into the model's (or an ancestor's) `textures` map — resolving the reference is M9-B05's |
| `cullface` | `Option<Direction>` | absent = never culled by an adjacent block |
| `rotation` | `u32` | default `0`; expected ∈ {0,90,180,270} |
| `tintindex` | `i32` | default `-1` (`-1` = no tint) |

### Caching and invalidation (per this task's own scope line)

`AssetCache` is keyed by a `StackFingerprint` — a cheap (mtime + byte-size per pack root, not a full content hash) signature over the *entire resolved stack in order*, not per individual file. Any change to which packs are enabled, their order, or a tracked pack root's own mtime/size invalidates the whole cache at once — this is deliberately coarser than per-file invalidation (matching CLIENT-D14's "baked once per resource-pack load" framing: a pack change is expected to be a rare, load-time event, not a per-frame concern, so cheap-but-coarse is the right tradeoff over expensive-but-precise).

### Content-audit scanner seam (ASSET-D24)

Two independent halves, both required, neither alone sufficient:

1. **Structural rule (enforced by this blueprint's own Constraints, mechanically grep-checkable):** `rc-assets`'s source contains zero `include_bytes!`/`include_str!`/`env!("OUT_DIR")`-based static asset embedding, and no `build.rs` exists anywhere in the crate. This makes "no asset ever gets compiled into the binary" true by construction for this crate specifically.
2. **Release-artifact scanner (this blueprint's concrete contribution to ASSET-D24's binding CI rule):** a new `xtask content-audit <path>` verb, added to the WS-D9 verb surface, that walks a given directory (or scans a single file) and fails closed (non-zero exit, listing every hit) if it finds: a PNG file-signature (`\x89PNG\r\n\x1a\n`), an OGG file-signature (`OggS`), a ZIP/JAR file-signature (`PK\x03\x04`) at a path whose extension is `.jar`, or a bare `.png`/`.ogg`/`.jar` file extension regardless of content. This is deliberately a superset check (extension **or** magic bytes, either one is a hit) — a release build with zero legitimate reason to ship any of those extensions can afford false positives far more than it can afford a missed real one.

### Public API dependency additions to `rc-assets/Cargo.toml`

M0-B01 scaffolded `rc-assets` with only `rc-core` and `rc-registries` path dependencies (the crate-graph edges `12-workspace-structure.md` fixes) and zero external dependencies. This blueprint adds exactly these external, already-`[workspace.dependencies]`-pinned crates (no new pin, no unpinned version — WS-D7):

```toml
[dependencies]
rc-core = { path = "../core" }
rc-registries = { path = "../registries" }
serde = { workspace = true }
serde_json = { workspace = true }
image = { workspace = true }
zip = { workspace = true }
sha1 = { workspace = true }
thiserror = { workspace = true }
```

`rc-core`/`rc-registries` remain present-but-unused-by-this-blueprint's-own-code (see Scope boundary above) — the edges stay because the crate-graph diagram in `12-workspace-structure.md` fixes them; a later blueprint (M9-B05) is what actually calls into `rc-registries`.

### Closing note: the non-gating real-jar verification pass

Everything this blueprint's CI tier checks runs against hand-authored fixtures (Acceptance tests, below) — no real Mojang content, no real vanilla jar, ever enters the repository or CI (TEST-D47/ASSET-D13). Separately, and **not** part of this blueprint's done-definition, an implementer with their own legally-owned 26.2 installation should manually run `rc-assets`'s discovery/parse pipeline against it once, as a real-world sanity pass — the same one-time, human-consent-gated, dev-machine-only category of check `09-testing-quality.md`'s oracle-bootstrap process establishes for the differential-testing tiers (TEST-D41), just informal here since this crate has no oracle-comparison content to formalize into a CI tier.

## Deliverables

### `crates/assets/src/resource_location.rs`

```rust
/// A namespaced identifier `namespace:path` (e.g. `minecraft:block/stone`), matching vanilla's
/// own identifier string format used throughout blockstate/model/texture references. A bare
/// string with no `:` is shorthand for the `minecraft` namespace (vanilla's own parsing rule).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceLocation {
    pub namespace: String,
    pub path: String,
}

impl ResourceLocation {
    /// Parses `"namespace:path"` or bare `"path"` (⇒ namespace `"minecraft"`).
    pub fn parse(s: &str) -> Self;
    /// Renders back to `"namespace:path"`.
    pub fn as_string(&self) -> String;
    /// The jar/pack-relative asset path for a blockstate: `assets/<namespace>/blockstates/<path>.json`.
    pub fn blockstate_path(&self) -> String;
    /// The jar/pack-relative asset path for a model: `assets/<namespace>/models/<path>.json`.
    pub fn model_path(&self) -> String;
    /// The jar/pack-relative asset path for a texture: `assets/<namespace>/textures/<path>.png`.
    pub fn texture_path(&self) -> String;
}
```

### `crates/assets/src/discovery.rs`

```rust
/// The pinned protocol target's version id (NET-D1) — `"26.2"`. The single source of truth this
/// whole crate checks the local installation against.
pub const PINNED_VERSION_ID: &str = "26.2";

/// Per-OS standard `.minecraft` root candidate (pure function, no filesystem access).
pub fn standard_locations() -> Vec<std::path::PathBuf>;

/// A fully validated, ready-to-read local installation for the pinned version.
#[derive(Debug, Clone)]
pub struct Installation {
    pub root: std::path::PathBuf,
    pub version_id: String,
    pub client_jar: std::path::PathBuf,
    pub version_manifest_path: std::path::PathBuf,
    pub asset_index_id: String,
    pub asset_index_path: std::path::PathBuf,
    pub assets_objects_dir: std::path::PathBuf,
    pub resourcepacks_dir: std::path::PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("no .minecraft installation found; probed: {probed:?}")]
    NoInstallationFound { probed: Vec<std::path::PathBuf> },
    #[error("custom path {0} is not a directory")]
    CustomPathNotADirectory(std::path::PathBuf),
    #[error("pinned version {expected} not found under {versions_dir:?}; launch it once via the official Minecraft Launcher")]
    MissingPinnedVersion { expected: &'static str, versions_dir: std::path::PathBuf },
    #[error("version manifest at {path:?} is not valid JSON or its \"id\" field does not match {expected:?}: {cause}")]
    CorruptVersionManifest { path: std::path::PathBuf, expected: &'static str, cause: String },
    #[error("asset index {id:?} referenced by the version manifest is missing at {path:?}")]
    MissingAssetIndex { id: String, path: std::path::PathBuf },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Discovers and validates an installation. `custom_path`, if given, is probed exclusively
/// (no fallback to standard locations); otherwise every `standard_locations()` candidate is
/// probed in order and the first that validates wins.
pub fn discover(custom_path: Option<&std::path::Path>) -> Result<Installation, DiscoveryError>;

/// Same as `discover`, but takes the candidate root list explicitly — the seam
/// `discover(None, ..)` calls internally, and the seam this blueprint's discovery-matrix
/// tests drive directly against fixture directories instead of real per-OS env vars.
pub fn discover_at(candidates: &[std::path::PathBuf]) -> Result<Installation, DiscoveryError>;
```

### `crates/assets/src/version_manifest.rs`

```rust
#[derive(Debug, serde::Deserialize)]
pub struct AssetIndexRef {
    pub id: String,
    pub url: String,
    pub sha1: String,
    pub size: u64,
    #[serde(rename = "totalSize")]
    pub total_size: u64,
}

#[derive(Debug, serde::Deserialize)]
pub struct DownloadEntry {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct DownloadsSection {
    pub client: DownloadEntry,
}

#[derive(Debug, serde::Deserialize)]
pub struct VersionManifest {
    pub id: String,
    #[serde(rename = "assetIndex")]
    pub asset_index: AssetIndexRef,
    pub downloads: DownloadsSection,
}

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub fn parse_version_manifest(bytes: &[u8]) -> Result<VersionManifest, ManifestError>;
```

### `crates/assets/src/jar.rs`

```rust
/// A read-only handle onto the pinned version's client jar, treated exclusively as a zip data
/// container (ASSET-D18/D19) — no `.class` entry is ever read or executed.
pub struct ClientJar { /* private: zip::ZipArchive<std::io::BufReader<std::fs::File>> */ }

#[derive(Debug, thiserror::Error)]
pub enum JarError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("zip archive error: {0}")]
    Zip(String),
    #[error("entry {0:?} not found in jar")]
    EntryNotFound(String),
}

impl ClientJar {
    pub fn open(path: &std::path::Path) -> Result<Self, JarError>;
    /// Reads one entry's full bytes. `jar_path` is jar-root-relative, e.g.
    /// `"assets/minecraft/blockstates/oak_door.json"` or `"pack.mcmeta"`.
    pub fn read_bytes(&mut self, jar_path: &str) -> Result<Vec<u8>, JarError>;
    /// True if `jar_path` names a real entry, without reading it.
    pub fn contains(&mut self, jar_path: &str) -> bool;
    /// Every entry path starting with `prefix` (e.g. `"assets/minecraft/models/block/"`).
    pub fn list_entries(&self, prefix: &str) -> Vec<String>;
}
```

### `crates/assets/src/asset_index.rs`

```rust
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AssetIndexObject {
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AssetIndex {
    pub objects: std::collections::HashMap<String, AssetIndexObject>,
}

#[derive(Debug, thiserror::Error)]
pub enum AssetIndexError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub fn load_asset_index(path: &std::path::Path) -> Result<AssetIndex, AssetIndexError>;

#[derive(Debug, thiserror::Error)]
pub enum ObjectResolveError {
    #[error("{0:?} is not a key in the asset index")]
    NotInIndex(String),
    #[error("object file missing at {0:?} (ASSET-D13: never re-fetched)")]
    MissingFile(std::path::PathBuf),
    #[error("SHA-1 mismatch at {path:?}: index declares {expected}, file hashes to {actual}")]
    HashMismatch { path: std::path::PathBuf, expected: String, actual: String },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Resolves `logical_path` (an `AssetIndex.objects` key) to its on-disk object under
/// `objects_dir`, verifying SHA-1 (ASSET-D14) before returning. Never repairs a mismatch.
pub fn resolve_object(
    objects_dir: &std::path::Path,
    index: &AssetIndex,
    logical_path: &str,
) -> Result<std::path::PathBuf, ObjectResolveError>;
```

### `crates/assets/src/resourcepack.rs`

```rust
/// Flattened plain-text `pack.mcmeta` description — rich text-component styling is discarded
/// (out of M9 scope, CLIENT-D17 is M10's).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatRange { pub min_inclusive: i32, pub max_inclusive: i32 }

#[derive(Debug, Clone)]
pub struct OverlayEntry { pub formats: FormatRange, pub directory: String }

#[derive(Debug, Clone)]
pub struct PackMeta {
    pub pack_format: i32,
    pub description: String,
    pub supported_formats: Option<FormatRange>,
    pub overlays: Vec<OverlayEntry>,
    /// Set by `build_stack` against `Installation`'s live-read base jar format — never computed
    /// by the parser itself, since the parser has no reference format to compare against.
    pub format_compatible: Option<bool>,
}

#[derive(Debug, thiserror::Error)]
pub enum PackMetaError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub fn parse_pack_meta(bytes: &[u8]) -> Result<PackMeta, PackMetaError>;

/// One entry in a caller-supplied, already-ordered enabled-pack list (lowest to highest
/// priority) — see Context's "Stacking order" for why this crate never parses `options.txt`.
#[derive(Debug, Clone)]
pub enum PackRef {
    /// `<resourcepacks_dir>/<name>/` (a directory pack).
    Directory(String),
    /// `<resourcepacks_dir>/<name>.zip` (an archive pack).
    Archive(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackSource {
    /// The base game's own assets, read from the client jar. Always index 0, always present.
    Vanilla,
    Directory(std::path::PathBuf),
    Archive(std::path::PathBuf),
}

#[derive(Debug, Clone)]
pub struct ResolvedPack {
    pub source: PackSource,
    /// `None` only for `PackSource::Vanilla`, which carries no pack.mcmeta of its own in this
    /// crate's model (its identity is implicit, not file-backed).
    pub meta: Option<PackMeta>,
}

/// The full, priority-ordered stack: index 0 is always `Vanilla`; increasing index = increasing
/// priority; the last entry wins any direct path conflict.
pub struct ResourceStack {
    pub packs: Vec<ResolvedPack>,
    /* private: an open ClientJar for Vanilla, plus per-Archive open zip::ZipArchive handles,
       kept alive for the stack's lifetime to avoid re-opening on every path lookup */
}

#[derive(Debug, thiserror::Error)]
pub enum PackStackError {
    #[error("pack {0:?} not found under resourcepacks/")]
    PackNotFound(String),
    #[error(transparent)]
    Jar(#[from] JarError),
    #[error(transparent)]
    Meta(#[from] PackMetaError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Builds the priority-ordered stack from `installation`'s client jar plus `enabled`, in the
/// order given (see Context's "Stacking order").
pub fn build_stack(
    installation: &crate::discovery::Installation,
    enabled: &[PackRef],
) -> Result<ResourceStack, PackStackError>;

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("{0:?} not found in any pack in the stack")]
    NotFound(String),
    #[error(transparent)]
    Jar(#[from] JarError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl ResourceStack {
    /// Reads `asset_path` (e.g. `"assets/minecraft/blockstates/oak_door.json"`), walking the
    /// stack from highest to lowest priority; returns the first pack that contains it, and
    /// which pack won (for diagnostics/cache-key purposes).
    pub fn read_bytes(&mut self, asset_path: &str) -> Result<(Vec<u8>, usize), ResolveError>;
    /// Every distinct path under `prefix` across the whole stack (deduplicated — a path present
    /// in more than one pack appears once, resolved to its winning pack's bytes on read).
    pub fn list_paths(&mut self, prefix: &str) -> Vec<String>;
    /// A cheap, order-sensitive signature of the whole stack's current identity (Context's
    /// caching section) — mtime+size per pack root, not a full content hash.
    pub fn fingerprint(&self) -> crate::cache::StackFingerprint;
}
```

### `crates/assets/src/texture.rs`

```rust
#[derive(Debug, Clone)]
pub struct DecodedTexture {
    pub width: u32,
    pub height: u32,
    /// Row-major, top-to-bottom, RGBA8 — `width * height * 4` bytes.
    pub rgba8: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum TextureError {
    #[error("PNG decode failed: {0}")]
    Decode(String),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub fn decode_png(bytes: &[u8]) -> Result<DecodedTexture, TextureError>;

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(untagged)]
pub enum AnimationFrame {
    Index(u32),
    Explicit { index: u32, time: Option<u32> },
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct AnimationMeta {
    #[serde(default)]
    pub interpolate: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
    #[serde(default = "crate::texture::default_frametime")]
    pub frametime: u32,
    #[serde(default)]
    pub frames: Vec<AnimationFrame>,
}
pub fn default_frametime() -> u32 { 1 }

#[derive(Debug, Clone, serde::Deserialize)]
pub struct McmetaFile {
    pub animation: Option<AnimationMeta>,
}

pub fn parse_mcmeta(bytes: &[u8]) -> Result<McmetaFile, TextureError>;

#[derive(Debug, Clone)]
pub struct ParsedTexture {
    pub id: crate::resource_location::ResourceLocation,
    pub image: DecodedTexture,
    pub animation: Option<AnimationMeta>,
}
```

### `crates/assets/src/blockstate.rs`

```rust
use std::collections::HashMap;
use crate::resource_location::ResourceLocation;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ModelRef {
    pub model: String,
    #[serde(default)] pub x: i32,
    #[serde(default)] pub y: i32,
    #[serde(default)] pub z: i32,
    #[serde(default)] pub uvlock: bool,
    #[serde(default = "crate::blockstate::default_weight")] pub weight: u32,
}
pub fn default_weight() -> u32 { 1 }

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum VariantValue {
    Single(ModelRef),
    Weighted(Vec<ModelRef>),
}

pub type PropertyMap = HashMap<String, String>;

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(untagged)]
pub enum WhenClause {
    Or { #[serde(rename = "OR")] or: Vec<PropertyMap> },
    And { #[serde(rename = "AND")] and: Vec<PropertyMap> },
    Flat(PropertyMap),
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct MultipartCase {
    pub when: Option<WhenClause>,
    pub apply: VariantValue,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RawBlockstate {
    pub variants: Option<HashMap<String, VariantValue>>,
    pub multipart: Option<Vec<MultipartCase>>,
}

#[derive(Debug, thiserror::Error)]
pub enum BlockstateParseError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub fn parse_blockstate(bytes: &[u8]) -> Result<RawBlockstate, BlockstateParseError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationIssue {
    BothVariantsAndMultipartPresent,
    NeitherVariantsNorMultipartPresent,
    UnknownRotationValue { context: String, field: &'static str, value: i32 },
    InvalidModelReference { context: String, raw: String },
}

/// Structural/range checks only — no property-set validation against `rc-registries` (M9-B05's
/// job, requires the block-state ID space this crate deliberately does not depend on).
pub fn validate_blockstate(raw: &RawBlockstate) -> Vec<ValidationIssue>;

/// Resolves `ModelRef.model` into a `ResourceLocation`, flagging (not rejecting) any `x`/`y`/`z`
/// outside `{0,90,180,270}`.
pub fn validate_model_ref(context: &str, r: &ModelRef) -> (ResourceLocation, Vec<ValidationIssue>);
```

### `crates/assets/src/model.rs`

```rust
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction { Down, Up, North, South, West, East }

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Axis { X, Y, Z }

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RawDisplayTransform {
    #[serde(default)] pub rotation: [f32; 3],
    #[serde(default)] pub translation: [f32; 3],
    #[serde(default = "crate::model::one_vec3")] pub scale: [f32; 3],
}
pub fn one_vec3() -> [f32; 3] { [1.0, 1.0, 1.0] }

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RawRotation {
    pub origin: [f32; 3],
    pub axis: Axis,
    pub angle: f32,
    #[serde(default)] pub rescale: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RawFace {
    pub uv: Option<[f32; 4]>,
    pub texture: String,
    pub cullface: Option<Direction>,
    #[serde(default)] pub rotation: u32,
    #[serde(default = "crate::model::neg_one")] pub tintindex: i32,
}
pub fn neg_one() -> i32 { -1 }

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RawElement {
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub rotation: Option<RawRotation>,
    #[serde(default = "crate::model::true_default")] pub shade: bool,
    #[serde(default)] pub light_emission: u8,
    pub faces: HashMap<Direction, RawFace>,
}
pub fn true_default() -> bool { true }

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RawModel {
    pub parent: Option<String>,
    #[serde(default, rename = "ambientocclusion")] pub ambient_occlusion: Option<bool>,
    pub textures: Option<HashMap<String, String>>,
    pub elements: Option<Vec<RawElement>>,
    pub display: Option<HashMap<String, RawDisplayTransform>>,
    pub gui_light: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinParent { Generated, Entity, Missing }
impl BuiltinParent {
    pub fn try_from_str(s: &str) -> Option<Self>;
}

#[derive(Debug, thiserror::Error)]
pub enum ModelParseError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub fn parse_model(bytes: &[u8]) -> Result<RawModel, ModelParseError>;

#[derive(Debug, Clone, PartialEq)]
pub enum ModelValidationIssue {
    CoordinateOutOfRange { element_index: usize, field: &'static str, axis: usize, value: f32 },
    UnusualRotationAngle { element_index: usize, angle: f32 },
    UnknownGuiLight { value: String },
    UnusualFaceRotation { element_index: usize, direction: Direction, value: u32 },
}

/// Structural/range checks only. Never resolves `#variable` texture references or the `parent`
/// chain (M9-B05's job).
pub fn validate_model(raw: &RawModel) -> Vec<ModelValidationIssue>;
```

### `crates/assets/src/cache.rs`

```rust
use std::collections::HashMap;
use std::sync::Arc;
use crate::resource_location::ResourceLocation;
use crate::{blockstate::RawBlockstate, model::RawModel, texture::ParsedTexture};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StackFingerprint(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheState { Fresh, Invalidated }

#[derive(Default)]
pub struct AssetCache {
    fingerprint: Option<StackFingerprint>,
    blockstates: HashMap<ResourceLocation, Arc<RawBlockstate>>,
    models: HashMap<ResourceLocation, Arc<RawModel>>,
    textures: HashMap<ResourceLocation, Arc<ParsedTexture>>,
}

impl AssetCache {
    pub fn new() -> Self;
    /// Compares `current` against the last-seen fingerprint; a mismatch clears every cached
    /// entry and records `current` as the new baseline. Returns which happened.
    pub fn sync(&mut self, current: StackFingerprint) -> CacheState;
    pub fn get_blockstate(&self, id: &ResourceLocation) -> Option<Arc<RawBlockstate>>;
    pub fn insert_blockstate(&mut self, id: ResourceLocation, v: RawBlockstate) -> Arc<RawBlockstate>;
    pub fn get_model(&self, id: &ResourceLocation) -> Option<Arc<RawModel>>;
    pub fn insert_model(&mut self, id: ResourceLocation, v: RawModel) -> Arc<RawModel>;
    pub fn get_texture(&self, id: &ResourceLocation) -> Option<Arc<ParsedTexture>>;
    pub fn insert_texture(&mut self, id: ResourceLocation, v: ParsedTexture) -> Arc<ParsedTexture>;
}
```

### `crates/assets/src/store.rs`

```rust
use std::sync::Arc;
use crate::resource_location::ResourceLocation;
use crate::{
    blockstate::RawBlockstate, cache::AssetCache, discovery::Installation, model::RawModel,
    resourcepack::ResourceStack, texture::ParsedTexture,
};

/// Top-level facade: discovery + resource-pack stack + parsed-asset cache, combined into one
/// load-by-id surface. This is the type a later blueprint (rc-render / M9-B01, M9-B05) actually
/// holds and calls.
pub struct AssetStore {
    pub installation: Installation,
    stack: ResourceStack,
    cache: AssetCache,
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error(transparent)]
    Resolve(#[from] crate::resourcepack::ResolveError),
    #[error(transparent)]
    BlockstateParse(#[from] crate::blockstate::BlockstateParseError),
    #[error(transparent)]
    ModelParse(#[from] crate::model::ModelParseError),
    #[error(transparent)]
    Texture(#[from] crate::texture::TextureError),
    #[error(transparent)]
    IndexObject(#[from] crate::asset_index::ObjectResolveError),
}

impl AssetStore {
    pub fn open(installation: Installation, stack: ResourceStack) -> Self;
    /// Re-checks the stack's fingerprint and clears the cache if it changed (Context's caching
    /// section) — call this whenever the caller's own pack-selection config may have changed.
    pub fn refresh(&mut self);
    pub fn load_blockstate(&mut self, id: &ResourceLocation) -> Result<Arc<RawBlockstate>, LoadError>;
    pub fn load_model(&mut self, id: &ResourceLocation) -> Result<Arc<RawModel>, LoadError>;
    pub fn load_texture(&mut self, id: &ResourceLocation) -> Result<Arc<ParsedTexture>, LoadError>;
    /// Generic hash-addressed asset-index object fetch (sound/lang/font/icon bytes) — used by
    /// later, out-of-M9-scope blueprints; returns raw bytes only, no format-specific parsing.
    pub fn load_index_object(&self, logical_path: &str) -> Result<Vec<u8>, LoadError>;
}
```

### `crates/assets/src/lib.rs`

```rust
//! `rc-assets` — locates and parses the player's local `.minecraft` installation (client jar +
//! resource packs) into engine-usable, unbaked textures/blockstates/models at runtime. Never
//! embeds, bundles, or distributes any Mojang asset (ASSET-D13); see this crate's owning
//! blueprint, M9-B02, for the complete legal/technical framing.

pub mod resource_location;
pub mod discovery;
pub mod version_manifest;
pub mod jar;
pub mod asset_index;
pub mod resourcepack;
pub mod texture;
pub mod blockstate;
pub mod model;
pub mod cache;
pub mod store;
```

### `xtask/src/content_audit.rs`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit { pub path: std::path::PathBuf, pub reason: &'static str }

/// Walks `root` (a directory or a single file) and returns every forbidden-content hit
/// (ASSET-D24): a `.png`/`.ogg`/`.jar` extension, or a PNG/OGG/ZIP magic-byte signature at any
/// path. Pure function over the filesystem — no network, no CI-specific behavior baked in.
pub fn scan(root: &std::path::Path) -> std::io::Result<Vec<Hit>>;

/// CLI entry point for the `content-audit` verb: scan + print every hit + exit code.
pub fn run(root: &std::path::Path) -> std::process::ExitCode;
```

`xtask/src/main.rs`'s `Command` enum (already `Debug, PartialEq` per M0-B01) gains one new variant:

```rust
/// ASSET-D24 release-artifact content scan.
ContentAudit { path: std::path::PathBuf },
```

## Acceptance tests (write these FIRST — own changeset)

**Changeset boundary:** the test-authoring changeset is every file under `crates/assets/tests/` (including `crates/assets/tests/fixtures/**` and `crates/assets/tests/fixtures/MANIFEST.json`) plus `xtask/tests/content_audit.rs`, committed together with every `crates/assets/src/*.rs` / `xtask/src/content_audit.rs` function body from the Deliverables section stubbed `todo!()` (or, for types, left fully defined — only function *bodies* are stubbed, since the tests must compile against the real signatures). The implementation changeset fills in bodies only; it must not touch anything under `tests/` or the fixtures/manifest.

### Fixture corpus (hand-authored, never Mojang-derived — TEST-D47 custody)

All fixtures live under `crates/assets/tests/fixtures/` and are loaded at test-run time via `std::fs` relative to `env!("CARGO_MANIFEST_DIR")` — never `include_bytes!` (Constraints). `crates/assets/tests/fixtures/MANIFEST.json` records one row per fixture file, matching TEST-D47's four-column shape: `{ "path": <relative path>, "sha256": <hex>, "generator": "M9-B02 fixture-authoring script v1, hand-run once per Implementation step 16", "source_vanilla_jar_hash": null }` — `source_vanilla_jar_hash` is `null` for every row in this manifest (not merely omitted), the explicit, checkable expression of "this fixture was never derived from any vanilla jar," standing in for TEST-D47's "source vanilla-jar hash it was derived from" column in the one case that column is inapplicable by design. A required test (`fixture_manifest_matches`, below) recomputes every listed file's SHA-256 and fails on any mismatch, mirroring TEST-D47's own mechanism at this crate's own scale.

Fixture tree (every texture inside it is a tiny, synthetically-generated 2×2 or 4×4 solid-color PNG, produced once by the fixture-authoring script in Implementation step 16 via the already-pinned `image`/`zip` crates and then committed as a static binary file like any other fixture — not a real Minecraft texture in any way, and not regenerated on every test run):

```
fixtures/
├── MANIFEST.json
├── fake_minecraft_root/                      # a synthetic ".minecraft" for discovery tests
│   ├── versions/26.2/26.2.jar                 # a zip containing:
│   │                                          #   pack.mcmeta {"pack":{"pack_format":99,"description":"fixture base pack"}}
│   │                                          #   assets/minecraft/blockstates/rc_test_block.json
│   │                                          #   assets/minecraft/models/block/rc_test_cube.json
│   │                                          #   assets/minecraft/models/block/rc_test_parent.json
│   │                                          #   assets/minecraft/textures/block/rc_test_stone.png (4x4 gray)
│   │                                          #   assets/minecraft/textures/block/rc_test_anim.png (4x4x3-frames)
│   │                                          #   assets/minecraft/textures/block/rc_test_anim.png.mcmeta
│   ├── versions/26.2/26.2.json                 # version manifest, id "26.2", assetIndex.id "test_index"
│   ├── assets/indexes/test_index.json          # {"objects": {"minecraft/fake/object.txt": {"hash": "<sha1 of a known payload>", "size": N}}}
│   ├── assets/objects/<hash[0:2]>/<hash>       # the object the index above points at
│   └── resourcepacks/
│       ├── override_pack/                      # a directory pack
│       │   ├── pack.mcmeta                      # {"pack":{"pack_format":99,"description":"override"}}
│       │   └── assets/minecraft/textures/block/rc_test_stone.png (4x4 RED — overrides the base)
│       └── archive_pack.zip                     # a zip pack containing:
│                                                 #   pack.mcmeta {"pack":{"pack_format":99,"description":"archive"}}
│                                                 #   assets/minecraft/blockstates/rc_test_block.json (different content — highest priority when enabled last)
├── incomplete_minecraft_root/                   # versions/26.2/26.2.json exists, .jar does NOT (MissingPinnedVersion case)
├── no_pinned_version_root/                      # versions/ exists but no "26.2" subdirectory at all
├── corrupt_manifest_root/versions/26.2/26.2.json  # malformed JSON
├── model_fixtures/
│   ├── valid_variants.json                     # blockstate: variants only, single + weighted-array cases
│   ├── valid_multipart.json                     # blockstate: multipart with when/OR/AND
│   ├── both_present.json                        # blockstate: variants AND multipart both set (invalid)
│   ├── neither_present.json                      # blockstate: {} (invalid)
│   ├── valid_model_with_elements.json             # model: parent, textures, elements w/ faces+rotation
│   ├── model_builtin_generated.json                # model: {"parent": "builtin/generated", "textures": {...}}
│   └── model_unusual_rotation.json                  # model: one element rotation.angle = 30 (flagged, not rejected)
└── mcmeta_fixtures/
    ├── anim_plain_int_frames.json                 # {"animation": {"frames": [0,1,2]}}
    ├── anim_mixed_frames.json                      # {"animation": {"frames": [0, {"index":1,"time":5}]}}
    └── anim_no_frames_key.json                      # {"animation": {"frametime": 2}} — frames omitted
```

### `crates/assets/tests/discovery_tests.rs`

| Test | Setup | Assertion |
|---|---|---|
| `standard_locations_returns_one_path_per_os` | none (pure function) | `standard_locations().len() >= 1`; on the compiling OS, the platform-appropriate suffix (`".minecraft"` on Windows/Linux, `"minecraft"` under `Library/Application Support` on macOS) is present in at least one candidate |
| `discovers_valid_fixture_root` | `discover_at(&[fixtures/fake_minecraft_root])` | `Ok(Installation { version_id, .. })` with `version_id == "26.2"`, `asset_index_id == "test_index"`, and every path field pointing at real files under the fixture |
| `custom_path_not_a_directory_errors` | `discover(Some(&fixtures/MANIFEST.json))` (a file, not a dir) | `Err(DiscoveryError::CustomPathNotADirectory(_))` |
| `missing_jar_is_missing_pinned_version` | `discover_at(&[fixtures/incomplete_minecraft_root])` | `Err(DiscoveryError::MissingPinnedVersion { .. })` |
| `no_versions_subdir_is_missing_pinned_version` | `discover_at(&[fixtures/no_pinned_version_root])` | `Err(DiscoveryError::MissingPinnedVersion { .. })` |
| `corrupt_manifest_errors_with_cause` | `discover_at(&[fixtures/corrupt_manifest_root])` | `Err(DiscoveryError::CorruptVersionManifest { cause, .. })` where `cause` is non-empty |
| `no_installation_found_lists_every_probed_path` | `discover_at(&[<two nonexistent temp paths>])` | `Err(DiscoveryError::NoInstallationFound { probed })` with `probed.len() == 2` |
| `first_valid_candidate_wins` | `discover_at(&[<nonexistent>, fixtures/fake_minecraft_root])` | `Ok(_)` — proves the "probe in order, first valid wins" contract |

### `crates/assets/tests/jar_tests.rs`

| Test | Setup | Assertion |
|---|---|---|
| `reads_known_entry` | open `fake_minecraft_root`'s jar, `read_bytes("assets/minecraft/blockstates/rc_test_block.json")` | `Ok(bytes)`, bytes parse as valid JSON via `serde_json::from_slice::<serde_json::Value>` |
| `missing_entry_errors` | `read_bytes("assets/minecraft/does_not_exist.json")` | `Err(JarError::EntryNotFound(_))` |
| `lists_entries_by_prefix` | `list_entries("assets/minecraft/textures/block/")` | contains exactly `rc_test_stone.png` and `rc_test_anim.png` (and its `.mcmeta`), nothing outside the prefix |
| `open_nonexistent_jar_errors` | `ClientJar::open(<nonexistent path>)` | `Err(JarError::Io(_))` |

### `crates/assets/tests/asset_index_tests.rs`

| Test | Setup | Assertion |
|---|---|---|
| `parses_valid_index` | `load_asset_index(fixtures/.../assets/indexes/test_index.json)` | `Ok(index)`, `index.objects.len() == 1`, the one entry's `hash`/`size` match the fixture |
| `malformed_json_errors` | a fixture file containing `"{not json"` | `Err(AssetIndexError::Json(_))` |
| `resolve_object_success` | `resolve_object` against the real fixture object file | `Ok(path)` pointing at the real on-disk object |
| `resolve_object_not_in_index` | `resolve_object(.., "minecraft/never/existed.txt")` | `Err(ObjectResolveError::NotInIndex(_))` |
| `resolve_object_missing_file` | index references a hash whose object file was deliberately not created in the fixture | `Err(ObjectResolveError::MissingFile(_))` |
| `resolve_object_hash_mismatch` | a fixture object file whose bytes were mutated after the index's `hash` was computed | `Err(ObjectResolveError::HashMismatch { expected, actual, .. })` with `expected != actual` |

### `crates/assets/tests/resourcepack_tests.rs`

| Test | Setup | Assertion |
|---|---|---|
| `pack_meta_parses_string_description` | `{"pack":{"pack_format":1,"description":"hello"}}` | `description == "hello"`, `supported_formats.is_none()` |
| `pack_meta_parses_text_component_description` | `{"pack":{"pack_format":1,"description":{"text":"hello ","extra":[{"text":"world"}]}}}` | `description == "hello world"` (flattened, styling discarded) |
| `pack_meta_parses_supported_formats_object` | `{"pack":{"pack_format":1,"supported_formats":{"min_inclusive":1,"max_inclusive":5},"description":""}}` | `supported_formats == Some(FormatRange{min_inclusive:1,max_inclusive:5})` |
| `pack_meta_parses_supported_formats_single_int` | `{"pack":{"pack_format":1,"supported_formats":3,"description":""}}` | `supported_formats == Some(FormatRange{min_inclusive:3,max_inclusive:3})` |
| `build_stack_vanilla_only` | `build_stack(installation, &[])` | `packs.len() == 1`, `packs[0].source == PackSource::Vanilla` |
| `build_stack_directory_and_archive` | `build_stack(installation, &[PackRef::Directory("override_pack"), PackRef::Archive("archive_pack.zip")])` | `packs.len() == 3`, in that exact order |
| `unknown_pack_ref_errors` | `build_stack(installation, &[PackRef::Directory("does_not_exist")])` | `Err(PackStackError::PackNotFound(_))` |
| `highest_priority_pack_wins_direct_override` | stack with `override_pack` enabled; `read_bytes("assets/minecraft/textures/block/rc_test_stone.png")` | returns `override_pack`'s RED bytes, not the base gray ones — assert by decoding and checking pixel color |
| `falls_through_to_lower_priority_when_not_overridden` | same stack; `read_bytes("assets/minecraft/blockstates/rc_test_block.json")` (only `override_pack` is enabled, which doesn't ship this path) | returns Vanilla's bytes |
| `last_of_two_overrides_wins` | stack with both `override_pack` then `archive_pack.zip` enabled in that order (archive is last ⇒ highest priority) for `rc_test_block.json` (archive overrides it, override_pack does not) | returned bytes match `archive_pack.zip`'s content |
| `not_found_anywhere_errors` | `read_bytes("assets/minecraft/nothing/here.json")` | `Err(ResolveError::NotFound(_))` |
| `fingerprint_stable_across_rebuild_of_identical_stack` | build the same stack twice from the same fixture files | both fingerprints equal |
| `fingerprint_changes_when_pack_content_changes` | build stack, note fingerprint, overwrite `override_pack/pack.mcmeta`'s bytes (changing mtime+size), rebuild | fingerprints differ |

### `crates/assets/tests/texture_tests.rs`

| Test | Setup | Assertion |
|---|---|---|
| `decodes_valid_png` | a synthetic 4×4 solid-color PNG built in-test via the `image` crate | `Ok(DecodedTexture { width: 4, height: 4, .. })`, `rgba8.len() == 64`, every pixel matches the known color |
| `decode_garbage_errors` | `decode_png(b"not a png")` | `Err(TextureError::Decode(_))` |
| `mcmeta_plain_int_frames` | `fixtures/mcmeta_fixtures/anim_plain_int_frames.json` | `frames == [Index(0), Index(1), Index(2)]` |
| `mcmeta_mixed_frames` | `fixtures/mcmeta_fixtures/anim_mixed_frames.json` | `frames[0] == Index(0)`, `frames[1] == Explicit { index: 1, time: Some(5) }` |
| `mcmeta_frames_omitted_defaults_empty` | `fixtures/mcmeta_fixtures/anim_no_frames_key.json` | `frames.is_empty()`, `frametime == 2` |
| `mcmeta_frametime_defaults_to_one` | `{"animation": {}}` | `frametime == 1`, `interpolate == false` |
| `mcmeta_no_animation_key_parses_none` | `{}` | `McmetaFile { animation: None }` |

### `crates/assets/tests/blockstate_tests.rs`

| Test | Setup | Assertion |
|---|---|---|
| `parses_single_variant` | `fixtures/model_fixtures/valid_variants.json`, one key | `VariantValue::Single(ModelRef { model, .. })` with the expected model string |
| `parses_weighted_variant_array` | same fixture, a different key | `VariantValue::Weighted(v)` with `v.len() == 2` and the expected weights |
| `parses_multipart_flat_when` | `fixtures/model_fixtures/valid_multipart.json`, a case with `"when": {"north": "true"}` | `WhenClause::Flat(map)` with `map["north"] == "true"` |
| `parses_multipart_or` | same fixture, an `"OR"` case | `WhenClause::Or { or }` with `or.len() == 2` |
| `multipart_case_with_no_when_always_applies` | same fixture, first case (no `when`) | `case.when.is_none()` |
| `validate_flags_both_present` | `fixtures/model_fixtures/both_present.json` | `validate_blockstate` returns a `Vec` containing `ValidationIssue::BothVariantsAndMultipartPresent` |
| `validate_flags_neither_present` | `fixtures/model_fixtures/neither_present.json` | contains `ValidationIssue::NeitherVariantsNorMultipartPresent` |
| `validate_clean_file_has_zero_issues` | `valid_variants.json` | `validate_blockstate(&raw).is_empty()` |
| `model_ref_flags_unusual_rotation` | a `ModelRef { y: 45, .. }` (not in `{0,90,180,270}`) | `validate_model_ref` returns an issue list containing `UnknownRotationValue { field: "y", value: 45, .. }` |
| `resource_location_parse_bare_defaults_to_minecraft` | `ResourceLocation::parse("block/stone")` | `namespace == "minecraft"`, `path == "block/stone"` |
| `resource_location_parse_namespaced` | `ResourceLocation::parse("rc_test:block/foo")` | `namespace == "rc_test"`, `path == "block/foo"` |

### `crates/assets/tests/model_tests.rs`

| Test | Setup | Assertion |
|---|---|---|
| `parses_model_with_elements_and_faces` | `fixtures/model_fixtures/valid_model_with_elements.json` | `elements` has ≥1 entry; its `faces` map contains at least `Direction::Up` and `Direction::North`; each face's `texture` starts with `#` |
| `parses_rotation_object` | same fixture, an element with a `rotation` block | `rotation.axis == Axis::Y`, `rotation.origin` matches the fixture's numbers |
| `parses_builtin_generated_parent` | `fixtures/model_fixtures/model_builtin_generated.json` | `BuiltinParent::try_from_str(raw.parent.as_deref().unwrap()) == Some(BuiltinParent::Generated)` |
| `non_builtin_parent_returns_none` | a model with `"parent": "minecraft:block/cube_all"` | `BuiltinParent::try_from_str(..) == None` |
| `defaults_shade_true_ambient_occlusion_none` | a minimal element with no `shade` key | `shade == true` |
| `default_tintindex_is_negative_one` | a face with no `tintindex` key | `tintindex == -1` |
| `validate_flags_unusual_angle` | `fixtures/model_fixtures/model_unusual_rotation.json` (`angle: 30`) | `validate_model` returns `ModelValidationIssue::UnusualRotationAngle { angle: 30.0, .. }` |
| `validate_clean_model_has_zero_issues` | `valid_model_with_elements.json` | `validate_model(&raw).is_empty()` |
| `unknown_face_direction_key_is_a_parse_error` | a model whose `faces` object has key `"sideways"` | `Err(ModelParseError::Json(_))` (serde rejects it — `Direction` has no such variant) |

### `crates/assets/tests/cache_tests.rs`

| Test | Setup | Assertion |
|---|---|---|
| `insert_then_get_returns_same_arc_content` | `cache.insert_model(id.clone(), model)`; `cache.get_model(&id)` | `Some(arc)` whose contents equal the inserted value |
| `sync_with_same_fingerprint_is_fresh_and_keeps_entries` | insert an entry, `sync(fp)`, `sync(fp)` again (same value) | second call returns `CacheState::Fresh`; the earlier entry is still present |
| `sync_with_new_fingerprint_invalidates` | insert an entry, `sync(fp1)`, then `sync(fp2)` with `fp2 != fp1` | returns `CacheState::Invalidated`; `get_model` for the earlier id is now `None` |

### `crates/assets/tests/store_tests.rs` (end-to-end integration)

| Test | Setup | Assertion |
|---|---|---|
| `full_pipeline_loads_blockstate_model_and_texture` | `AssetStore::open` over `discover_at(fake_minecraft_root)` + `build_stack(.., &[])` | `load_blockstate(&ResourceLocation::parse("rc_test_block"))`, `load_model(&ResourceLocation::parse("rc_test_cube"))`, `load_texture(&ResourceLocation::parse("block/rc_test_stone"))` all `Ok`, and a second call to each returns the *same* `Arc` pointer (proves cache hit, no re-parse) |
| `refresh_after_pack_change_invalidates` | load a texture, mutate `override_pack`'s texture file on disk, rebuild the stack with `override_pack` enabled, `store.refresh()`, reload | the second load's pixel content differs from the first (proves the cache actually picked up the pack change, not a stale entry) |
| `load_index_object_returns_matching_bytes` | `load_index_object("minecraft/fake/object.txt")` | `Ok(bytes)` matching the fixture object's known content |

### `crates/assets/tests/fixture_manifest_test.rs`

| Test | Setup | Assertion |
|---|---|---|
| `fixture_manifest_matches` | read `MANIFEST.json`, recompute SHA-256 of every listed file | every recomputed hash equals the manifest's recorded hash; the manifest lists every file actually present under `fixtures/` (no orphans in either direction) |

### `xtask/tests/content_audit.rs`

| Test | Setup | Assertion |
|---|---|---|
| `clean_directory_has_zero_hits` | a temp dir containing only `.rs`/`.toml`/`.json` files | `scan(dir) == Ok(vec![])` |
| `flags_png_extension` | temp dir with a zero-byte file named `texture.png` | `scan` returns one `Hit` naming that path |
| `flags_ogg_extension` | temp dir with `sound.ogg` | one `Hit` |
| `flags_jar_extension` | temp dir with `bundled.jar` | one `Hit` |
| `flags_png_magic_bytes_regardless_of_extension` | temp dir with a file named `data.bin` whose first 8 bytes are the real PNG signature | one `Hit` |
| `run_exits_nonzero_on_hit` | `run(&dir_with_a_png)` | `ExitCode` compares equal to `ExitCode::FAILURE` |
| `run_exits_zero_on_clean_dir` | `run(&clean_dir)` | `ExitCode::SUCCESS` |
| `scans_single_file_argument` | `scan(&path_to_one_png_file)` (not a directory) | one `Hit` for that exact path |

## Implementation steps

1. **Cargo manifest.** Add the dependency block from Context's "Public API dependency additions" to `crates/assets/Cargo.toml`. Observable: `cargo build -p rc-assets` compiles (empty modules).
2. **`resource_location.rs`.** Implement `ResourceLocation::parse`/`as_string`/the three path-helper methods. Pure string logic, no I/O.
3. **`version_manifest.rs`.** Implement `parse_version_manifest` via `serde_json::from_slice`.
4. **`discovery.rs`.** Implement `standard_locations` (per-OS `#[cfg]` branches reading `std::env::var("APPDATA")` on Windows, `std::env::var("HOME")` + the fixed macOS/Linux suffixes elsewhere). Implement `discover_at`: for each candidate, check `<root>/versions/26.2/26.2.jar` and `.json` exist; parse the manifest via step 3, confirm `id == "26.2"`; compute `asset_index_path` from the manifest's `asset_index.id`, confirm it exists; populate and return `Installation` on the first candidate that fully validates, else the appropriate `DiscoveryError`. Implement `discover` as `discover_at(&custom_path.map(|p| vec![p.to_owned()]).unwrap_or_else(standard_locations))`, with the `CustomPathNotADirectory` pre-check when `custom_path` is `Some`.
5. **`jar.rs`.** Implement `ClientJar::open` (`zip::ZipArchive::new` over a `BufReader<File>`), `read_bytes` (`archive.by_name(jar_path)`, read fully, map "not found" to `EntryNotFound`), `contains`, `list_entries` (iterate `archive.file_names()`, filter by prefix).
6. **`asset_index.rs`.** Implement `load_asset_index` (read file, `serde_json::from_slice`). Implement `resolve_object`: look up `logical_path` in `index.objects` (else `NotInIndex`); compute the on-disk path `objects_dir/<hash[0:2]>/<hash>`; if absent, `MissingFile`; else read it, hash via `sha1::Sha1::digest`, hex-encode lowercase, compare to the declared `hash` (else `HashMismatch`).
7. **`resourcepack.rs`.** Implement `parse_pack_meta` (including the text-component-to-plain-string flattening: recursively read every `text` field of the root object plus its `extra` array, concatenating in order — a small, self-contained ~15-line recursive helper, no external text-component crate). Implement `build_stack`: always start with `PackSource::Vanilla` (`meta: None`, `format_compatible: None` — the base is the format reference itself); for each `PackRef`, resolve to a real path under `installation.resourcepacks_dir` (directory or `<name>.zip`), else `PackNotFound`; for an `Archive`, open its `zip::ZipArchive`; read and parse that pack's own `pack.mcmeta` (jar root for Vanilla — actually Vanilla has no separate meta call here per the type; instead read the **client jar's own root `pack.mcmeta`** separately as `Installation`'s live base-format reference, used only to set every *other* pack's `format_compatible` flag, not stored on the `Vanilla` entry itself). Implement `ResourceStack::read_bytes`/`list_paths` walking `self.packs` from `len()-1` down to `0`, checking `contains`/`by_name` on each until a hit. Implement `fingerprint`: for each pack's backing file/dir, `std::fs::metadata` → `(len, mtime)`, fold into one `u64` via a simple order-sensitive combining hash (e.g. FNV-1a over the concatenated little-endian bytes of every `(len, mtime_secs)` pair in stack order — deterministic, no new dependency needed).
8. **`texture.rs`.** Implement `decode_png` via `image::load_from_memory` → `.to_rgba8()` → extract `width()`/`height()`/`into_raw()`. Implement `parse_mcmeta` via `serde_json::from_slice::<McmetaFile>`.
9. **`blockstate.rs`.** Implement `parse_blockstate` via `serde_json::from_slice`. Implement `validate_blockstate` (the both/neither checks). Implement `validate_model_ref` (parse `ResourceLocation`, check `x`/`y`/`z` membership in `{0,90,180,270}`).
10. **`model.rs`.** Implement `parse_model` via `serde_json::from_slice`. Implement `BuiltinParent::try_from_str` (exact string match against the three sentinels). Implement `validate_model` (range checks on `from`/`to` against −16..=32, `angle` against the discrete set per the moderate-confidence resolution above, `gui_light` against `{"front","side"}`, face `rotation` against `{0,90,180,270}`).
11. **`cache.rs`.** Implement `AssetCache::sync`/the per-type get/insert pairs — straightforward `HashMap` operations plus the fingerprint-compare-and-clear-all-three-maps logic in `sync`.
12. **`store.rs`.** Implement `AssetStore::open`/`refresh` (calls `stack.fingerprint()` then `cache.sync`). Implement `load_blockstate`/`load_model` identically: cache hit → return the cached `Arc`; miss → `stack.read_bytes(id.<kind>_path())`, parse via the matching module's `parse_*`, insert into cache, return the new `Arc`. Implement `load_texture` the same way except it resolves **two** independent paths per Context's "PNG texture decoding" section: `stack.read_bytes(id.texture_path())` (required — `ResolveError::NotFound` propagates as `LoadError`) decoded via `texture::decode_png`, plus a second, best-effort `stack.read_bytes(&format!("{}.mcmeta", id.texture_path()))` whose `Err(ResolveError::NotFound(_))` is treated as "no animation" (`None`), never propagated as an error, while any other error (a real I/O failure) still propagates — parsed via `texture::parse_mcmeta` into `McmetaFile.animation` when present, combined into one `ParsedTexture { id, image, animation }`, then cached. Implement `load_index_object` via `asset_index::resolve_object` + `std::fs::read`.
13. **`lib.rs`.** Wire the `pub mod` list.
14. **`xtask/src/content_audit.rs`.** Implement `scan`: if `root` is a file, check it alone; if a directory, `walkdir`-style manual recursion via `std::fs::read_dir` (no new dependency — hand-rolled recursive walk); for each file, check its extension against `{"png","ogg","jar"}` case-insensitively, and read its first 8 bytes to compare against the three magic-byte signatures (PNG: `89 50 4E 47 0D 0A 1A 0A`; OGG: `4F 67 67 53`; ZIP/JAR: `50 4B 03 04`, only counted as a hit when the extension is `.jar`). Implement `run`: call `scan`, print each `Hit`, return `ExitCode::FAILURE` if non-empty else `ExitCode::SUCCESS`.
15. **`xtask/src/main.rs`.** Add the `ContentAudit { path }` variant to `Command` and its dispatch arm calling `content_audit::run(&path)`.
16. **Fixture authoring.** Build every file under `crates/assets/tests/fixtures/` by hand (synthetic JSON, synthetic PNGs generated via a small one-off script using the already-pinned `image`/`zip` crates — never sourced from any real Minecraft installation), then compute and write `MANIFEST.json`'s SHA-256 rows.
17. **Full test run.** `cargo nextest run -p rc-assets` and `cargo nextest run -p xtask -- content_audit` — every test from the Acceptance tests section now passes.

## Constraints & forbidden actions

(a) **Test-first changeset boundary is binding.** Every file under `crates/assets/tests/` and `xtask/tests/content_audit.rs`, plus the fixture corpus and its manifest, are committed first with every `crates/assets/src/*.rs` and `xtask/src/content_audit.rs` function body `todo!()`-stubbed. The implementation changeset (steps 1–17 above, minus step 16's fixture authoring, which belongs to the test-authoring changeset alongside the tests that consume it) fills in bodies only and must not edit any test file, fixture, or the manifest.

(b) **No new external dependencies beyond the pinned set.** `serde`, `serde_json`, `image`, `zip`, `sha1`, `thiserror` are all already in `12-workspace-structure.md`'s `[workspace.dependencies]` table — this blueprint wires them into `rc-assets` for the first time but adds no new pin. Do not add `walkdir`, `tempfile`, `anyhow`, `reqwest`, or any text-component/rich-text crate — the recursive `pack.mcmeta` description flattener, the directory walk in `content_audit::scan`, and every test's temp-directory handling are hand-rolled specifically to avoid needing any of these.

(c) **No Mojang or third-party reimplementation code.** The blockstate/model/mcmeta/asset-index/pack.mcmeta schemas in this blueprint's Context section were sourced from minecraft.wiki's public documentation (ASSET-D18(b)) during this blueprint's own derivation — no decompiled jar, no leaked source, no other reimplementation's code was consulted for any of them (ASSET-D18/D19/D30). Every fixture file is hand-authored from scratch for this blueprint, never copied or extracted from a real Minecraft installation (TEST-D47).

(d) **Absolute no-embedding rule (mechanically checkable, backs ASSET-D24).** No file under `crates/assets/src/` or `xtask/src/content_audit.rs` may contain `include_bytes!`, `include_str!`, or a `build.rs` — verified by `grep -rn 'include_bytes!\|include_str!' crates/assets/src xtask/src/content_audit.rs` returning nothing, and by the absence of any `crates/assets/build.rs`.

(e) **No network I/O anywhere in this crate.** `rc-assets`'s dependency list (item (b) above) contains no HTTP client — this is the structural enforcement of ASSET-D13's "never fetches" rule; do not add one, ever, in this blueprint or by extension.

(f) **No `unsafe` code.** Nothing in this blueprint's deliverables uses `unsafe`.

(g) **Scope boundary — do not implement beyond this blueprint's named surface.** No baking/interpretation logic (M9-B05), no animation playback or GPU upload (a later blueprint), no `sounds.json`/lang-file structural parsing (M10), no item-model-definition parsing (deferred, unassigned). Do not add any of these as a shortcut to "look more complete" — every type this blueprint ships stays raw/unbaked.

## Verification commands

Run from the workspace root on a clean checkout, on both Windows and Linux (TEST-D43 — nothing in this blueprint is oracle-dependent, so both legs run identically with no setup step):

```
cargo build -p rc-assets -p xtask
cargo clippy -p rc-assets -p xtask --all-targets -- -D warnings
cargo nextest run -p rc-assets
cargo nextest run -p xtask -- content_audit
cargo run -p xtask -- content-audit crates/assets/src
```

Expected: every command exits 0; the final `content-audit` invocation against this blueprint's own source tree reports zero hits (proving constraint (d) holds for real, not just by the grep in (d)'s own description). CI (Tier 1, TEST-D37) green is the authoritative done-signal (TEST-D50) — a local pass alone does not close this blueprint.
