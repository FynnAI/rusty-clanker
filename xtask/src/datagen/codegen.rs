//! Pure codegen (`generate`) plus the CLI-facing I/O wrapper (`run`) for the `codegen`
//! verb. See Context's "Determinism" subsection for the four rules `generate` must
//! follow — restated as doc comments on the function itself below.

use super::reports::{BlocksReport, RegistriesReport, find_default_state_id};

/// WS-D15 (M3.5-B01) §3.5 candidate replaceable-block list (local names, no
/// namespace) — the 28 non-snow members of the 26.2 `minecraft:replaceable` block
/// tag, verified by the TEST-D57 research pass against the local datagen export
/// `data/minecraft/tags/block/replaceable.json` (`blueprints/M3.5/M3.5-B01-
/// CLAIMS.md`) — not a reading of the wiki. Kept in the tag file's own order so a
/// re-verification is a plain diff. See this blueprint's own Claims-to-verify list
/// before trusting it for anything gameplay-relevant beyond what TEST-D57 already
/// confirmed.
pub const REPLACEABLE_BLOCKS: &[&str] = &[
    "air",
    "water",
    "lava",
    "short_grass",
    "fern",
    "dead_bush",
    "bush",
    "short_dry_grass",
    "tall_dry_grass",
    "seagrass",
    "tall_seagrass",
    "fire",
    "soul_fire",
    "vine",
    "glow_lichen",
    "resin_clump",
    "light",
    "tall_grass",
    "large_fern",
    "structure_void",
    "void_air",
    "cave_air",
    "bubble_column",
    "warped_roots",
    "nether_sprouts",
    "crimson_roots",
    "leaf_litter",
    "hanging_roots",
];

/// `true` iff `block_name` (local, unnamespaced) is `REPLACEABLE_BLOCKS`-listed, with
/// the one hand-coded exception: `"snow"` is replaceable only when `state_props`
/// contains `("layers", "1")` — WS-D15 §3.5. `minecraft:snow`'s own static
/// `replaceable` flag is actually `true` uniformly across all of its states (TEST-D57
/// research pass, `M3.5-B01-CLAIMS.md`); the `layers == 1`-only restriction modeled
/// here is the correct behavior for a generic state-only replaceable query, matching
/// vanilla's default (non-snow-item-in-hand) placement path exactly.
///
/// `pub`, not `pub(crate)` — `xtask/tests/datagen_block_state_properties_codegen.rs`'s
/// own `is_state_replaceable_*` cases call this directly (not through
/// `generate_block_state_properties_rs`), which requires external visibility from that
/// separate integration-test crate, mirroring `generate_registry_entries_rs`'s own
/// identical `pub`-for-external-test-visibility rationale earlier in this file.
pub fn is_state_replaceable(block_name: &str, state_props: &[(String, String)]) -> bool {
    if block_name == "snow" {
        return state_props.iter().any(|(k, v)| k == "layers" && v == "1");
    }
    REPLACEABLE_BLOCKS.contains(&block_name)
}

/// `xtask`'s own crate version, tagged as this codegen format's identity — written
/// into every `MANIFEST.json` entry's `generator_tool_version` field.
pub const CODEGEN_TOOL_VERSION: &str = concat!("xtask-codegen/", env!("CARGO_PKG_VERSION"));

pub struct GeneratedFiles {
    /// `(relative filename under crates/registries/generated/v<protocol_version>/, content)`,
    /// in write order: `("registries.rs", ...)`, `("block_states.rs", ...)`.
    pub files: Vec<(String, String)>,
}

fn strip_namespace(id: &str) -> &str {
    id.split_once(':').map(|(_, path)| path).unwrap_or(id)
}

/// `pub(crate)` — reused by `super::tags`' own generated module-name construction (M1
/// registry-sync-fix follow-up), for the identical slash/keyword-escaping transform applied
/// to a discovered registry's own directory path (e.g. `"worldgen/biome"` -> `"worldgen_biome"`).
pub(crate) fn sanitize_mod_name(path: &str) -> String {
    let mut s: String = path
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    if s.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        s.insert(0, '_');
    }
    if is_rust_keyword(&s) {
        s.push('_');
    }
    s
}

/// `pub(crate)` — reused by `super::tags`' own generated-identifier construction (M1
/// registry-sync-fix follow-up), which needs the identical alphanumeric-uppercase/`_`-escape
/// transform this module already applies to registry entry names, applied instead to tag
/// paths (e.g. `"enchantable/armor"` -> `"ENCHANTABLE_ARMOR"`).
pub(crate) fn sanitize_const_name(path: &str) -> String {
    // SCREAMING_SNAKE_CASE output never collides with a Rust keyword (keywords are
    // always lowercase), so no keyword guard is needed here. It CAN collide with
    // `generate_registries_rs`'s own reserved per-module `COUNT` aggregate constant —
    // real 26.2 data confirms this happens (`minecraft:worldgen/placement_modifier_type`
    // registers an entry literally named "count") — so that one reserved identifier
    // gets the same trailing-underscore escape `sanitize_mod_name` already uses for
    // keyword collisions.
    let mut s: String = path
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    if s.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        s.insert(0, '_');
    }
    if s == "COUNT" {
        s.push('_');
    }
    s
}

fn is_rust_keyword(s: &str) -> bool {
    matches!(
        s,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "try"
    )
}

fn generate_registries_rs(registries: &RegistriesReport) -> String {
    let mut out = String::new();
    out.push_str("//! Generated by `xtask codegen` — do not edit by hand, re-run instead.\n");
    out.push_str("//!\n");
    out.push_str(
        "//! NET-D9/NET-D10: registry entry ID<->name tables, derived from `--reports`'\n",
    );
    out.push_str(
        "//! `registries.json` as processed, code-generated Rust source (never raw Mojang JSON).\n\n",
    );
    out.push_str("#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]\n");
    out.push_str("pub struct RegistryEntryId(pub u32);\n\n");

    for (registry_name, report) in registries {
        let mod_name = sanitize_mod_name(strip_namespace(registry_name));
        let mut entries: Vec<(&String, u32)> = report
            .entries
            .iter()
            .map(|(name, entry)| (name, entry.protocol_id))
            .collect();
        entries.sort_by_key(|(_, id)| *id);

        out.push_str(&format!("pub mod {mod_name} {{\n"));
        out.push_str("    use super::RegistryEntryId;\n\n");
        for (entry_name, protocol_id) in &entries {
            let const_name = sanitize_const_name(strip_namespace(entry_name));
            out.push_str(&format!(
                "    pub const {const_name}: RegistryEntryId = RegistryEntryId({protocol_id});\n"
            ));
        }
        out.push_str(&format!(
            "\n    pub const COUNT: u32 = {};\n",
            entries.len()
        ));
        out.push_str("}\n\n");
    }

    out
}

fn generate_block_states_rs(blocks: &BlocksReport) -> String {
    let mut out = String::new();
    out.push_str("//! Generated by `xtask codegen` — do not edit by hand, re-run instead.\n");
    out.push_str("//!\n");
    out.push_str(
        "//! NET-D9/NET-D10: block-state ID tables, derived from `--reports`' `blocks.json`\n",
    );
    out.push_str("//! as processed, code-generated Rust source (never raw Mojang JSON).\n\n");
    out.push_str("#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]\n");
    out.push_str("pub struct BlockStateId(pub u32);\n\n");

    let block_state_count: usize = blocks.values().map(|b| b.states.len()).sum();
    out.push_str(&format!(
        "pub const BLOCK_TYPE_COUNT: u32 = {};\n",
        blocks.len()
    ));
    out.push_str(&format!(
        "pub const BLOCK_STATE_COUNT: u32 = {block_state_count};\n\n"
    ));

    out.push_str("pub mod default_state {\n");
    out.push_str("    use super::BlockStateId;\n\n");
    for (block_name, block) in blocks {
        let const_name = sanitize_const_name(strip_namespace(block_name));
        let default_id = find_default_state_id(block)
            .expect("every real block report entry has exactly one default state");
        out.push_str(&format!(
            "    pub const {const_name}: BlockStateId = BlockStateId({default_id});\n"
        ));
    }
    out.push_str("}\n");

    out
}

/// M1-B04's own hand-authored, minecraft.wiki-sourced (verified 2026-08-21) list of every
/// `WORLDGEN`-layer registry — the registries Registry Data (`0x07`) ever transmits, never
/// the protocol-version-fixed `STATIC`-layer registries `registries.rs` already covers.
/// Re-verify against the real fetched `reports/registries.json` before every codegen
/// re-run that adds or removes a registry (blueprint Constraints (f)) — not sacred.
pub const WORLDGEN_REGISTRIES: &[&str] = &[
    "minecraft:banner_pattern",
    "minecraft:chat_type",
    "minecraft:damage_type",
    "minecraft:dialog",
    "minecraft:dimension_type",
    "minecraft:enchantment",
    "minecraft:instrument",
    "minecraft:jukebox_song",
    "minecraft:painting_variant",
    "minecraft:sulfur_cube_archetype",
    "minecraft:test_environment",
    "minecraft:test_instance",
    "minecraft:timeline",
    "minecraft:trim_material",
    "minecraft:trim_pattern",
    "minecraft:world_clock",
    "minecraft:worldgen/biome",
    "minecraft:cat_variant",
    "minecraft:cat_sound_variant",
    "minecraft:chicken_variant",
    "minecraft:chicken_sound_variant",
    "minecraft:cow_variant",
    "minecraft:cow_sound_variant",
    "minecraft:frog_variant",
    "minecraft:pig_variant",
    "minecraft:pig_sound_variant",
    "minecraft:wolf_variant",
    "minecraft:wolf_sound_variant",
    "minecraft:zombie_nautilus_variant",
];

/// Pure: for each name in `WORLDGEN_REGISTRIES`, look up `registries.get(name)` — absent
/// panics naming the missing registry (a loud, implementation-time-only failure). Collects
/// `(entry_name, protocol_id)` pairs, sorts by `protocol_id` (identical determinism rule
/// to `generate_registries_rs`), and emits one `pub mod { pub const ENTRIES: &[&str] }` per
/// registry — entry strings written in full, unsanitized (they are string literals, not
/// identifiers, so need no identifier-safety transform, unlike `registries.rs`'s own
/// `sanitize_const_name`-based output) — followed by one closing
/// `pub static REGISTRIES: &[(&str, &[&str])]` in `WORLDGEN_REGISTRIES`'s own fixed order.
///
/// `pub`, not private — `xtask/tests/datagen_codegen.rs`'s own
/// `registry_entries_panics_on_missing_worldgen_registry` case calls this directly (not
/// through `generate`) to observe the missing-registry panic in isolation, which requires
/// external visibility from that separate integration-test crate.
pub fn generate_registry_entries_rs(registries: &RegistriesReport) -> String {
    let mut out = String::new();
    out.push_str("//! Generated by `xtask codegen` — do not edit by hand, re-run instead.\n");
    out.push_str("//!\n");
    out.push_str("//! M1-B04: the exact original identifier string for every entry of every\n");
    out.push_str("//! WORLDGEN-layer registry, derived from `--reports`' `registries.json` as\n");
    out.push_str("//! processed, code-generated Rust source (never raw Mojang JSON).\n\n");

    for name in WORLDGEN_REGISTRIES {
        let report = registries
            .get(*name)
            .unwrap_or_else(|| panic!("WORLDGEN_REGISTRIES names {name:?}, absent from the real registries report — re-verify the list against reports/registries.json"));
        let mut entries: Vec<(&String, u32)> = report
            .entries
            .iter()
            .map(|(entry_name, entry)| (entry_name, entry.protocol_id))
            .collect();
        entries.sort_by_key(|(_, id)| *id);

        let mod_name = sanitize_mod_name(strip_namespace(name));
        out.push_str(&format!("pub mod {mod_name} {{\n"));
        out.push_str("    pub const ENTRIES: &[&str] = &[\n");
        for (entry_name, _) in &entries {
            out.push_str(&format!("        {entry_name:?},\n"));
        }
        out.push_str("    ];\n");
        out.push_str("}\n\n");
    }

    out.push_str("pub static REGISTRIES: &[(&str, &[&str])] = &[\n");
    for name in WORLDGEN_REGISTRIES {
        let mod_name = sanitize_mod_name(strip_namespace(name));
        out.push_str(&format!("    ({name:?}, {mod_name}::ENTRIES),\n"));
    }
    out.push_str("];\n");

    out
}

/// WS-D15 (M3.5-B01): pure, `blocks: &BlocksReport` in, `block_state_properties.rs`'s
/// full source text out — every block's own state-id range/default state
/// (`BLOCK_RANGES`), every state's own resolved property list/owning block/
/// `replaceable` flag (`STATE_PROPERTIES`/`STATE_BLOCK`/`STATE_REPLACEABLE`), and a
/// per-block, property-sorted state index (`BLOCK_STATE_INDEX`) for `state_id`'s own
/// binary search. State ids and property values are read straight from each block's
/// own `states[]` entries (§3.2 — the report's own generation order), never
/// recomputed from the block-level `properties` value-list cartesian product. See
/// Implementation step 8 for the exact emission algorithm.
fn format_property_tuple_list(props: &[(String, String)]) -> String {
    let mut s = String::from("&[");
    for (i, (k, v)) in props.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        s.push_str(&format!("(\"{k}\", \"{v}\")"));
    }
    s.push(']');
    s
}

pub fn generate_block_state_properties_rs(blocks: &BlocksReport) -> String {
    let mut out = String::new();
    out.push_str("//! Generated by `xtask codegen` — do not edit by hand, re-run instead.\n");
    out.push_str(
        "//! WS-D15: block-state property registry, derived from `--reports`' `blocks.json` (ids,\n",
    );
    out.push_str(
        "//! properties) and `xtask`'s own hand-authored `REPLACEABLE_BLOCKS` list — never raw\n",
    );
    out.push_str("//! Mojang JSON.\n\n");
    out.push_str("use super::block_states::BlockStateId;\n\n");
    out.push_str(
        "/// One block *type* (not a specific state): an index into the block table, in\n",
    );
    out.push_str(
        "/// `blocks.json`'s own key order — the same order `block_states.rs`'s `default_state`\n",
    );
    out.push_str("/// module declares its constants in.\n");
    out.push_str("#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]\n");
    out.push_str("pub struct BlockId(pub u32);\n\n");
    out.push_str("/// One block's full block-state id range plus its default state.\n");
    out.push_str("#[derive(Copy, Clone, Debug, PartialEq, Eq)]\n");
    out.push_str("pub struct BlockStateRange {\n");
    out.push_str("    pub first: BlockStateId,\n");
    out.push_str("    pub last: BlockStateId,\n");
    out.push_str("    pub default: BlockStateId,\n");
    out.push_str("}\n\n");
    out.push_str(
        "/// One state's resolved property list paired with its own state id — `BLOCK_STATE_INDEX`'s\n",
    );
    out.push_str("/// element type, sorted ascending by `.0` within each block's own row.\n");
    out.push_str(
        "pub type StateEntry = (&'static [(&'static str, &'static str)], BlockStateId);\n\n",
    );

    // Per-block computed data, in `blocks`' own (BTreeMap = alphabetical-by-full-name)
    // order — this fixes `BlockId` assignment order, matching `block_states.rs`'s own
    // `default_state` constant order (§3.2/§3.3).
    struct BlockData<'a> {
        full_name: &'a str,
        local_name: &'a str,
        const_name: String,
        block_id: u32,
        first: u32,
        last: u32,
        default: u32,
    }

    let mut block_datas: Vec<BlockData> = Vec::with_capacity(blocks.len());
    for (i, (full_name, block)) in blocks.iter().enumerate() {
        let local_name = strip_namespace(full_name);
        let const_name = sanitize_const_name(local_name);
        let first = block
            .states
            .iter()
            .map(|s| s.id)
            .min()
            .unwrap_or_else(|| panic!("block {full_name} has no states"));
        let last = block
            .states
            .iter()
            .map(|s| s.id)
            .max()
            .unwrap_or_else(|| panic!("block {full_name} has no states"));
        let width = block.states.len() as u32;
        assert_eq!(
            last - first + 1,
            width,
            "block {full_name}: state ids {first}..={last} are not contiguous with its own {width} states \
             (malformed report — this defense is never expected to fire against a real report)"
        );
        let default = find_default_state_id(block)
            .unwrap_or_else(|| panic!("block {full_name} has no state flagged default"));
        block_datas.push(BlockData {
            full_name,
            local_name,
            const_name,
            block_id: i as u32,
            first,
            last,
            default,
        });
    }

    out.push_str("pub mod block_id {\n");
    out.push_str("    use super::BlockId;\n");
    for bd in &block_datas {
        out.push_str(&format!(
            "    pub const {}: BlockId = BlockId({});\n",
            bd.const_name, bd.block_id
        ));
    }
    out.push_str("}\n\n");

    out.push_str("/// Indexed by `BlockId.0`; length `block_states::BLOCK_TYPE_COUNT`.\n");
    out.push_str("pub static BLOCK_RANGES: &[BlockStateRange] = &[\n");
    for bd in &block_datas {
        out.push_str(&format!(
            "    BlockStateRange {{ first: BlockStateId({}), last: BlockStateId({}), default: BlockStateId({}) }},\n",
            bd.first, bd.last, bd.default
        ));
    }
    out.push_str("];\n\n");

    // Dense flat tables, indexed by raw state id. Sized by the maximum observed id + 1
    // (never a separately-computed "sum of per-block state counts") — for a real
    // report the two coincide exactly (§3.2: the combined id space is globally
    // contiguous from 0, zero gaps across all ~1196 blocks), so this is the same
    // length as `block_states::BLOCK_STATE_COUNT`; sizing by the data's own observed
    // maximum, rather than trusting that coincidence blindly, is what keeps this
    // function panic-free against any future non-degenerate fixture.
    let max_id = blocks
        .values()
        .flat_map(|b| b.states.iter().map(|s| s.id))
        .max()
        .unwrap_or(0);
    let flat_len = (max_id + 1) as usize;
    let mut flat: Vec<Option<(String, u32, bool)>> =
        std::iter::repeat_with(|| None).take(flat_len).collect();
    for bd in &block_datas {
        let block = &blocks[bd.full_name];
        for state in &block.states {
            let props_text = format_property_tuple_list(&state.properties.0);
            let replaceable = is_state_replaceable(bd.local_name, &state.properties.0);
            flat[state.id as usize] = Some((props_text, bd.block_id, replaceable));
        }
    }

    out.push_str(
        "/// Dense, indexed directly by raw state id; length `block_states::BLOCK_STATE_COUNT`.\n",
    );
    out.push_str("pub static STATE_PROPERTIES: &[&[(&str, &str)]] = &[\n");
    for entry in &flat {
        match entry {
            Some((props, _, _)) => out.push_str(&format!("    {props},\n")),
            None => out.push_str("    &[],\n"),
        }
    }
    out.push_str("];\n\n");

    out.push_str("/// Dense, indexed directly by raw state id; same length.\n");
    out.push_str("pub static STATE_BLOCK: &[BlockId] = &[\n");
    for entry in &flat {
        match entry {
            Some((_, block_id, _)) => out.push_str(&format!("    BlockId({block_id}),\n")),
            None => out.push_str("    BlockId(0),\n"),
        }
    }
    out.push_str("];\n\n");

    out.push_str("/// Dense, indexed directly by raw state id; same length.\n");
    out.push_str("pub static STATE_REPLACEABLE: &[bool] = &[\n");
    for entry in &flat {
        match entry {
            Some((_, _, replaceable)) => out.push_str(&format!("    {replaceable},\n")),
            None => out.push_str("    false,\n"),
        }
    }
    out.push_str("];\n\n");

    out.push_str(
        "/// Indexed by `BlockId.0`; length `block_states::BLOCK_TYPE_COUNT`. Row `b` holds every\n",
    );
    out.push_str("/// one of `b`'s own states, sorted ascending by `StateEntry.0`.\n");
    out.push_str("pub static BLOCK_STATE_INDEX: &[&[StateEntry]] = &[\n");
    for bd in &block_datas {
        let block = &blocks[bd.full_name];
        let mut rows: Vec<(Vec<(String, String)>, u32)> = block
            .states
            .iter()
            .map(|s| (s.properties.0.clone(), s.id))
            .collect();
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        out.push_str("    &[\n");
        for (props, id) in &rows {
            out.push_str(&format!(
                "        ({}, BlockStateId({id})),\n",
                format_property_tuple_list(props)
            ));
        }
        out.push_str("    ],\n");
    }
    out.push_str("];\n");

    out
}

/// Pure transform: `--reports` data in, generated Rust source out. No filesystem
/// access. Deterministic per Context's four rules: parses/iterates only via `BTreeMap`
/// (never `HashMap`), sorts registry entries by `protocol_id` explicitly, embeds no
/// timestamp anywhere, and sanitizes identifiers as a pure function of the input
/// string alone. Two logically-identical `RegistriesReport`/`BlocksReport` values
/// (even if built via `.insert()` calls in different orders) MUST produce byte-
/// identical `GeneratedFiles::files` content — this is the property
/// `output_is_independent_of_input_insertion_order` (Acceptance tests) checks.
pub fn generate(registries: &RegistriesReport, blocks: &BlocksReport) -> GeneratedFiles {
    GeneratedFiles {
        files: vec![
            (
                "registries.rs".to_string(),
                generate_registries_rs(registries),
            ),
            (
                "block_states.rs".to_string(),
                generate_block_states_rs(blocks),
            ),
            (
                "registry_entries.rs".to_string(),
                generate_registry_entries_rs(registries),
            ),
            (
                "block_state_properties.rs".to_string(),
                generate_block_state_properties_rs(blocks),
            ),
        ],
    }
}

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

fn read_report_file(dir: &std::path::Path, name: &str, version: &str) -> Result<String, String> {
    let path = dir.join(name);
    std::fs::read_to_string(&path).map_err(|_| {
        format!(
            "missing {name} under {} — run `cargo xtask fetch-data {version}` first",
            dir.display()
        )
    })
}

/// `block_state_properties.rs`'s own raw emission (Deliverables §4.3) is a
/// one-entry-per-line, but never full-file rustfmt-clean, text (its nested tuple/
/// struct-literal array rows are not hand-formatted to rustfmt's own wrapping rules,
/// unlike `registries.rs`/`block_states.rs`/`registry_entries.rs`'s simple one-const-
/// per-line shape). Reformats in place via a real `rustfmt` invocation, mirroring
/// `datagen/tags.rs::run`'s own identical "format before hashing" precedent (same
/// module doc comment there explains why), so the committed file and its manifest
/// entry always agree on the actual on-disk (post-format) bytes.
fn rustfmt_in_place(file_path: &std::path::Path) -> Result<(), String> {
    let status = std::process::Command::new("rustfmt")
        .arg("--edition")
        .arg("2024")
        .arg(file_path)
        .status()
        .map_err(|e| format!("failed to invoke rustfmt on {}: {e}", file_path.display()))?;
    if !status.success() {
        return Err(format!(
            "rustfmt exited with a failure status formatting {}",
            file_path.display()
        ));
    }
    Ok(())
}

/// I/O wrapper: reads `registries.json`+`blocks.json` from `args.reports_dir` (`Err`
/// naming the exact missing file and suggesting `cargo xtask fetch-data <version>` if
/// either is absent), calls `generate`, writes all four files plus `MANIFEST.json`
/// under `args.out_dir` (preserving any pre-existing manifest entry this call didn't
/// itself just (re)write — e.g. `codegen-tags`' own additive `tags.rs` entry, which
/// this verb never generates and must not silently drop), then immediately calls
/// `fixture_manifest::verify_manifest` against what it just wrote as a self-check
/// (defense against a write-time bug producing a manifest that does not actually
/// match the bytes on disk).
pub fn run(args: &CodegenArgs) -> Result<(), String> {
    let registries_json = read_report_file(&args.reports_dir, "registries.json", &args.mc_version)?;
    let blocks_json = read_report_file(&args.reports_dir, "blocks.json", &args.mc_version)?;

    let registries: RegistriesReport = serde_json::from_str(&registries_json)
        .map_err(|e| format!("failed to parse registries.json: {e}"))?;
    let blocks: BlocksReport = serde_json::from_str(&blocks_json)
        .map_err(|e| format!("failed to parse blocks.json: {e}"))?;

    let generated = generate(&registries, &blocks);

    std::fs::create_dir_all(&args.out_dir)
        .map_err(|e| format!("failed to create {}: {e}", args.out_dir.display()))?;

    let mut files_as_bytes: Vec<(String, Vec<u8>)> = Vec::with_capacity(generated.files.len());
    for (name, content) in &generated.files {
        let path = args.out_dir.join(name);
        std::fs::write(&path, content)
            .map_err(|e| format!("failed to write {}: {e}", path.display()))?;
        if name == "block_state_properties.rs" {
            rustfmt_in_place(&path)?;
        }
        let on_disk = std::fs::read(&path)
            .map_err(|e| format!("failed to re-read {}: {e}", path.display()))?;
        files_as_bytes.push((name.clone(), on_disk));
    }

    let mut manifest = crate::fixture_manifest::build_manifest(
        args.protocol_version,
        &args.mc_version,
        &files_as_bytes,
        CODEGEN_TOOL_VERSION,
        &args.source_jar_sha1,
    );
    let manifest_path = args.out_dir.join("MANIFEST.json");
    let just_written: std::collections::BTreeSet<&str> =
        files_as_bytes.iter().map(|(n, _)| n.as_str()).collect();
    if let Ok(existing_text) = std::fs::read_to_string(&manifest_path)
        && let Ok(existing) =
            serde_json::from_str::<crate::fixture_manifest::FixtureManifest>(&existing_text)
    {
        for entry in existing.entries {
            if !just_written.contains(entry.path.as_str()) {
                manifest.entries.push(entry);
            }
        }
    }
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("failed to serialize MANIFEST.json: {e}"))?;
    std::fs::write(&manifest_path, &manifest_json)
        .map_err(|e| format!("failed to write {}: {e}", manifest_path.display()))?;

    let violations = crate::fixture_manifest::verify_manifest(&manifest_path, &args.out_dir);
    if !violations.is_empty() {
        let details: Vec<String> = violations
            .iter()
            .map(|v| format!("{} [{}]: {}", v.path, v.kind, v.message))
            .collect();
        return Err(format!(
            "codegen self-check failed — the manifest it just wrote does not match the files it just wrote: {}",
            details.join("; ")
        ));
    }

    Ok(())
}
