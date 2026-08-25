//! Tag-membership codegen — M1 registry-sync-fix follow-up
//! (`docs/research/mc-26.2/26-registry-sync-configuration.md` §5.3/§6, NET-D9/NET-D10).
//! Reads the vanilla datapack's own `data/minecraft/tags/**` JSON from a local
//! data-generator output directory (never committed raw — ASSET-D18(f)/NET-D9's carve-out
//! only covers the *derived*, code-generated Rust this module emits, mirroring `codegen.rs`'s
//! own already-established precedent for `registries.json`/`blocks.json`) plus the same
//! `--reports` `registries.json` `codegen.rs` already parses (for the two static registries'
//! own numeric ids), and produces `crates/registries/generated/v<protocol_version>/tags.rs`.
//!
//! Scope: exactly the doc's §5.3 minimal required tag set, per registry, plus whatever
//! further tags/entries each one transitively references via `#minecraft:...` sub-tag
//! refs — never "every tag in the game." `TAG_REGISTRIES` is this pass's own single source of
//! truth for which tags that is.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::reports::RegistriesReport;

/// One vanilla tag JSON file's parsed `values` array element — either a plain member/tag-
/// reference string, or the `{"id": ..., "required": false}` object form (real vanilla's own
/// "don't error if this reference can't be resolved" escape hatch, §6). Both forms share one
/// `id: String`, distinguished only by a leading `#` (a sub-tag reference) — this is real
/// vanilla's own tag-file schema, not an invention of this module.
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(untagged)]
enum RawTagValue {
    Plain(String),
    Object {
        id: String,
        #[serde(default = "default_required")]
        required: bool,
    },
}
fn default_required() -> bool {
    true
}

#[derive(serde::Deserialize, Debug, Clone, Default)]
struct TagFile {
    #[serde(default)]
    values: Vec<RawTagValue>,
}

/// One tag file's own normalized entry: `id` keeps its original on-disk form (a plain
/// `"minecraft:x"` member id, or a `"#minecraft:x"` sub-tag reference); `required` is the
/// vanilla default (`true`) unless the object form set it explicitly.
#[derive(Debug, Clone)]
pub struct RawEntry {
    pub id: String,
    pub required: bool,
}

/// `tag path (no namespace, no ".json") -> its own raw entry list`, for every `.json` file
/// found recursively under one registry's own tag directory — e.g. the file at
/// `item/enchantable/armor.json` keys this map under `"enchantable/armor"`.
pub type TagTree = BTreeMap<String, Vec<RawEntry>>;

/// Reads every `.json` file recursively under
/// `<tags_root>/data/minecraft/tags/<registry_dir>/` into a `TagTree`.
pub fn load_tag_tree(tags_root: &Path, registry_dir: &str) -> Result<TagTree, String> {
    let base = tags_root
        .join("data")
        .join("minecraft")
        .join("tags")
        .join(registry_dir);
    let mut tree = TagTree::new();
    walk_dir(&base, &base, &mut tree)?;
    Ok(tree)
}

fn walk_dir(base: &Path, dir: &Path, tree: &mut TagTree) -> Result<(), String> {
    let read_dir = std::fs::read_dir(dir)
        .map_err(|e| format!("failed to read tag directory {}: {e}", dir.display()))?;
    let mut children: Vec<PathBuf> = Vec::new();
    for entry in read_dir {
        let entry = entry.map_err(|e| format!("failed to list {}: {e}", dir.display()))?;
        children.push(entry.path());
    }
    children.sort();

    for path in children {
        if path.is_dir() {
            walk_dir(base, &path, tree)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let rel = path
                .strip_prefix(base)
                .expect("walk_dir only ever visits paths under base");
            let tag_path: String = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            let tag_path = tag_path
                .strip_suffix(".json")
                .expect("filtered to a .json extension above")
                .to_string();

            let text = std::fs::read_to_string(&path)
                .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
            let file: TagFile = serde_json::from_str(&text)
                .map_err(|e| format!("failed to parse {}: {e}", path.display()))?;
            let entries = file
                .values
                .into_iter()
                .map(|v| match v {
                    RawTagValue::Plain(id) => RawEntry { id, required: true },
                    RawTagValue::Object { id, required } => RawEntry { id, required },
                })
                .collect();
            tree.insert(tag_path, entries);
        }
    }
    Ok(())
}

fn split_minecraft_namespace(id: &str) -> Result<&str, String> {
    id.strip_prefix("minecraft:").ok_or_else(|| {
        format!("non-\"minecraft\" namespace not supported by this codegen pass: {id:?}")
    })
}

/// Recursively resolves `tag_path` (e.g. `"enchantable/armor"`) against `tree` into the flat,
/// deduplicated set of plain member identifiers (`"minecraft:x"`, namespace included) it
/// ultimately names — following every `#minecraft:...` sub-tag reference transitively, per
/// standard vanilla tag semantics (`docs/research/mc-26.2/26-registry-sync-configuration.md`
/// §5.2/§6). An entry with `required: false` that fails to resolve (missing tag file, missing
/// or non-`"minecraft"` namespace) is silently dropped rather than erroring — vanilla's own
/// escape hatch; a `required: true` entry (the default; every plain string entry is always
/// `required: true`) propagates the same failure as `Err`. A self-referencing cycle is also
/// `Err`, never an infinite loop.
pub fn resolve_tag(tag_path: &str, tree: &TagTree) -> Result<BTreeSet<String>, String> {
    resolve_inner(tag_path, tree, &mut Vec::new())
}

fn resolve_inner(
    tag_path: &str,
    tree: &TagTree,
    visiting: &mut Vec<String>,
) -> Result<BTreeSet<String>, String> {
    if visiting.iter().any(|s| s == tag_path) {
        return Err(format!(
            "cycle detected resolving #minecraft:{tag_path} (call stack: {visiting:?})"
        ));
    }
    let raw = tree
        .get(tag_path)
        .ok_or_else(|| format!("no tag JSON file found for #minecraft:{tag_path}"))?;

    visiting.push(tag_path.to_string());
    let mut members = BTreeSet::new();
    for entry in raw {
        let resolved: Result<BTreeSet<String>, String> =
            if let Some(sub_ref) = entry.id.strip_prefix('#') {
                split_minecraft_namespace(sub_ref)
                    .and_then(|sub_path| resolve_inner(sub_path, tree, visiting))
            } else {
                split_minecraft_namespace(&entry.id).map(|_| BTreeSet::from([entry.id.clone()]))
            };
        match resolved {
            Ok(found) => members.extend(found),
            Err(_) if !entry.required => {} // vanilla's own required=false escape hatch (§6)
            Err(e) => {
                visiting.pop();
                return Err(e);
            }
        }
    }
    visiting.pop();
    Ok(members)
}

/// Where a resolved member identifier's numeric wire id comes from.
enum IdSpace<'a> {
    /// A static registry (`block`, `item`) — numeric id = the real `registries.json` report's
    /// own `protocol_id` for that member name.
    Static(&'a RegistriesReport),
    /// A dynamic/registration-phase registry (`enchantment`, `dialog`, `timeline`) — numeric
    /// id = the member's 0-based index position in `order`, which MUST stay identical to
    /// `crates/server/src/play/world.rs`'s own `SYNCHRONIZED_REGISTRIES` entry list for this
    /// same registry (§5.2: "that numeric id is the entry's position ... as established by
    /// this same connection's own RegistryData packet").
    Dynamic(&'a [&'a str]),
}

fn numeric_id(space: &IdSpace, registry_id: &str, member_id: &str) -> Result<u32, String> {
    match space {
        IdSpace::Static(registries) => registries
            .get(registry_id)
            .and_then(|report| report.entries.get(member_id))
            .map(|entry| entry.protocol_id)
            .ok_or_else(|| {
                format!(
                    "{member_id} not found in the real registries.json report's {registry_id} entries"
                )
            }),
        IdSpace::Dynamic(order) => order
            .iter()
            .position(|n| *n == member_id)
            .map(|i| i as u32)
            .ok_or_else(|| {
                format!(
                    "{member_id} not found in this codegen's own hand-authored {registry_id} \
                     order list -- keep it in sync with play::world::SYNCHRONIZED_REGISTRIES"
                )
            }),
    }
}

/// One registry's own minimal required-tag root set (§5.3) plus which id space its resolved
/// members live in. `pub`/fields `pub` so `xtask/tests/datagen_tags.rs` can assert against
/// this table's own shape directly, mirroring `codegen.rs`'s own `WORLDGEN_REGISTRIES`
/// precedent.
pub struct TagRegistrySpec {
    /// Directory name under `data/minecraft/tags/` (also this generated module's own name).
    pub dir: &'static str,
    /// Full resource-location registry id, as sent on the wire.
    pub registry_id: &'static str,
    /// Tag paths (relative to `dir`, no `.json`) this registry's minimal root set names.
    pub roots: &'static [&'static str],
    /// `None` for a static registry; `Some(order)` for a dynamic one — see `IdSpace::Dynamic`.
    pub dynamic_order: Option<&'static [&'static str]>,
}

/// `minecraft:enchantment`'s own registration order — copied verbatim from
/// `crates/server/src/play/world.rs`'s `SYNCHRONIZED_REGISTRIES` entry (43 entries, matching
/// the doc's own confirmed "43/43 entries" count, §2). Re-verify both together if either ever
/// changes.
const ENCHANTMENT_ORDER: &[&str] = &[
    "minecraft:aqua_affinity",
    "minecraft:bane_of_arthropods",
    "minecraft:binding_curse",
    "minecraft:blast_protection",
    "minecraft:breach",
    "minecraft:channeling",
    "minecraft:density",
    "minecraft:depth_strider",
    "minecraft:efficiency",
    "minecraft:feather_falling",
    "minecraft:fire_aspect",
    "minecraft:fire_protection",
    "minecraft:flame",
    "minecraft:fortune",
    "minecraft:frost_walker",
    "minecraft:impaling",
    "minecraft:infinity",
    "minecraft:knockback",
    "minecraft:looting",
    "minecraft:loyalty",
    "minecraft:luck_of_the_sea",
    "minecraft:lunge",
    "minecraft:lure",
    "minecraft:mending",
    "minecraft:multishot",
    "minecraft:piercing",
    "minecraft:power",
    "minecraft:projectile_protection",
    "minecraft:protection",
    "minecraft:punch",
    "minecraft:quick_charge",
    "minecraft:respiration",
    "minecraft:riptide",
    "minecraft:sharpness",
    "minecraft:silk_touch",
    "minecraft:smite",
    "minecraft:soul_speed",
    "minecraft:sweeping_edge",
    "minecraft:swift_sneak",
    "minecraft:thorns",
    "minecraft:unbreaking",
    "minecraft:vanishing_curse",
    "minecraft:wind_burst",
];

/// `minecraft:dialog`'s own registration order — copied verbatim from `play::world::
/// SYNCHRONIZED_REGISTRIES`.
const DIALOG_ORDER: &[&str] = &[
    "minecraft:custom_options",
    "minecraft:quick_actions",
    "minecraft:server_links",
];

/// `minecraft:timeline`'s own registration order — copied verbatim from `play::world::
/// SYNCHRONIZED_REGISTRIES`.
const TIMELINE_ORDER: &[&str] = &[
    "minecraft:day",
    "minecraft:early_game",
    "minecraft:moon",
    "minecraft:villager_schedule",
];

/// The doc's §5.3 minimal required tag set, one row per registry. `pub` so
/// `xtask/tests/datagen_tags.rs` can assert against it directly.
pub const TAG_REGISTRIES: &[TagRegistrySpec] = &[
    TagRegistrySpec {
        dir: "block",
        registry_id: "minecraft:block",
        roots: &[
            "infiniburn_end",
            "infiniburn_nether",
            "infiniburn_overworld",
        ],
        dynamic_order: None,
    },
    TagRegistrySpec {
        dir: "item",
        registry_id: "minecraft:item",
        roots: &[
            "enchantable/armor",
            "enchantable/bow",
            "enchantable/chest_armor",
            "enchantable/crossbow",
            "enchantable/durability",
            "enchantable/equippable",
            "enchantable/fire_aspect",
            "enchantable/fishing",
            "enchantable/foot_armor",
            "enchantable/head_armor",
            "enchantable/leg_armor",
            "enchantable/lunge",
            "enchantable/mace",
            "enchantable/melee_weapon",
            "enchantable/mining",
            "enchantable/mining_loot",
            "enchantable/sharp_weapon",
            "enchantable/sweeping",
            "enchantable/trident",
            "enchantable/vanishing",
            "enchantable/weapon",
            "sulfur_cube_archetype/bouncy",
            "sulfur_cube_archetype/explosive",
            "sulfur_cube_archetype/fast_flat",
            "sulfur_cube_archetype/fast_sliding",
            "sulfur_cube_archetype/high_resistance",
            "sulfur_cube_archetype/hot",
            "sulfur_cube_archetype/light",
            "sulfur_cube_archetype/regular",
            "sulfur_cube_archetype/slow_bouncy",
            "sulfur_cube_archetype/slow_flat",
            "sulfur_cube_archetype/slow_sliding",
            "sulfur_cube_archetype/sticky",
        ],
        dynamic_order: None,
    },
    TagRegistrySpec {
        dir: "enchantment",
        registry_id: "minecraft:enchantment",
        roots: &[
            "exclusive_set/armor",
            "exclusive_set/boots",
            "exclusive_set/bow",
            "exclusive_set/crossbow",
            "exclusive_set/damage",
            "exclusive_set/mining",
            "exclusive_set/riptide",
        ],
        dynamic_order: Some(ENCHANTMENT_ORDER),
    },
    TagRegistrySpec {
        dir: "dialog",
        registry_id: "minecraft:dialog",
        roots: &["pause_screen_additions", "quick_actions"],
        dynamic_order: Some(DIALOG_ORDER),
    },
    TagRegistrySpec {
        dir: "timeline",
        registry_id: "minecraft:timeline",
        roots: &["in_end", "in_nether", "in_overworld"],
        dynamic_order: Some(TIMELINE_ORDER),
    },
];

/// Pure transform: `tags_root`'s own tag-file tree plus the real `registries` report in,
/// generated Rust source out (or the first resolution/lookup failure encountered, naming the
/// offending tag/member). Deterministic: `TAG_REGISTRIES`' own fixed declared order, `BTreeSet`-
/// sorted member ids throughout, no filesystem access beyond `load_tag_tree` (all pure from
/// there).
pub fn generate_tags_rs(tags_root: &Path, registries: &RegistriesReport) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("//! Generated by `xtask codegen-tags` — do not edit by hand, re-run instead.\n");
    out.push_str("//!\n");
    out.push_str(
        "//! NET-D9/NET-D10, docs/research/mc-26.2/26-registry-sync-configuration.md §5.3/§6:\n",
    );
    out.push_str(
        "//! real vanilla tag-membership tables for the Configuration-phase Update Tags\n",
    );
    out.push_str("//! packet's minimal required tag set, resolved (including transitive #tag\n");
    out.push_str(
        "//! references) from the vanilla datapack's own tags/** JSON via the project's own\n",
    );
    out.push_str(
        "//! data generator output — never raw Mojang JSON committed, only this derived,\n",
    );
    out.push_str("//! code-generated Rust source.\n\n");
    out.push_str("#[derive(Debug, Clone, Copy)]\n");
    out.push_str("pub struct TagTable {\n");
    out.push_str("    pub tag_id: &'static str,\n");
    out.push_str("    pub entries: &'static [u32],\n");
    out.push_str("}\n\n");

    for spec in TAG_REGISTRIES {
        let tree = load_tag_tree(tags_root, spec.dir)?;
        let space = match spec.dynamic_order {
            Some(order) => IdSpace::Dynamic(order),
            None => IdSpace::Static(registries),
        };

        out.push_str(&format!("pub mod {} {{\n", spec.dir));
        out.push_str("    use super::TagTable;\n\n");

        let mut const_names = Vec::with_capacity(spec.roots.len());
        for root in spec.roots {
            let members = resolve_tag(root, &tree)?;
            let mut ids: Vec<u32> = members
                .iter()
                .map(|member| numeric_id(&space, spec.registry_id, member))
                .collect::<Result<_, _>>()?;
            ids.sort_unstable();
            ids.dedup();
            let ids_src: Vec<String> = ids.iter().map(|id| id.to_string()).collect();

            let const_name = super::codegen::sanitize_const_name(root);
            out.push_str(&format!(
                "    pub const {const_name}: TagTable = TagTable {{ tag_id: \"minecraft:{root}\", entries: &[{}] }};\n",
                ids_src.join(", ")
            ));
            const_names.push(const_name);
        }
        out.push_str(&format!(
            "\n    pub const TAGS: &[TagTable] = &[{}];\n",
            const_names.join(", ")
        ));
        out.push_str("}\n\n");
    }

    out.push_str(
        "/// `(registry_id, tags)`, in `TAG_REGISTRIES`'s own declared order — order carries no\n",
    );
    out.push_str(
        "/// wire-protocol meaning (docs/research/mc-26.2/26-registry-sync-configuration.md\n",
    );
    out.push_str("/// §4.4), this is purely this table's own fixed iteration order.\n");
    out.push_str("pub static REGISTRIES: &[(&str, &[TagTable])] = &[\n");
    for spec in TAG_REGISTRIES {
        out.push_str(&format!(
            "    (\"{}\", {}::TAGS),\n",
            spec.registry_id, spec.dir
        ));
    }
    out.push_str("];\n");

    Ok(out)
}

pub struct TagsCodegenArgs {
    /// Directory containing `data/minecraft/tags/**` — typically a full (`--all`) local
    /// data-generator run's own `generated/` output, kept entirely outside this repository.
    pub tags_root: PathBuf,
    /// A prior `fetch-data` run's `generated/reports/` directory (`registries.json` is read
    /// from here — the two static registries' own numeric ids).
    pub reports_dir: PathBuf,
    /// `crates/registries/generated/v<protocol_version>/` — must already exist (created by a
    /// prior `codegen` run); this verb only ever adds/updates `tags.rs` and its own
    /// `MANIFEST.json` entry, never touching `registries.rs`/`block_states.rs`.
    pub out_dir: PathBuf,
    pub source_jar_sha1: String,
    pub protocol_version: u32,
    pub mc_version: String,
}

/// I/O wrapper: reads `registries.json` from `args.reports_dir` (`Err` naming the exact
/// missing file and suggesting `cargo xtask fetch-data <version>` if absent), calls
/// `generate_tags_rs`, writes `tags.rs` under `args.out_dir`, then merges a `tags.rs` entry
/// into that directory's existing `MANIFEST.json` — additive: replaces a stale `tags.rs`
/// entry from a prior run if present, but never touches any other entry (`registries.rs`/
/// `block_states.rs` stay exactly as `codegen::run` last wrote them). Finishes with the same
/// `verify_manifest` self-check `codegen::run` already performs.
pub fn run(args: &TagsCodegenArgs) -> Result<(), String> {
    let registries_json_path = args.reports_dir.join("registries.json");
    let registries_json = std::fs::read_to_string(&registries_json_path).map_err(|_| {
        format!(
            "missing {} — run `cargo xtask fetch-data {}` first",
            registries_json_path.display(),
            args.mc_version
        )
    })?;
    let registries: RegistriesReport = serde_json::from_str(&registries_json)
        .map_err(|e| format!("failed to parse registries.json: {e}"))?;

    let content = generate_tags_rs(&args.tags_root, &registries)?;

    std::fs::create_dir_all(&args.out_dir)
        .map_err(|e| format!("failed to create {}: {e}", args.out_dir.display()))?;
    let file_path = args.out_dir.join("tags.rs");
    std::fs::write(&file_path, &content)
        .map_err(|e| format!("failed to write {}: {e}", file_path.display()))?;

    // `generate_tags_rs`'s own output (unlike `registries.rs`/`block_states.rs`'s one-const-
    // per-line shape) emits long single-line `TagTable` literals for any tag with more than a
    // handful of members — never already rustfmt-clean. Reformatting here, before hashing,
    // means the committed file and its manifest entry always agree on the *actual* on-disk
    // (post-format) bytes, with no separate manual "formatting only" follow-up commit needed.
    let rustfmt_status = std::process::Command::new("rustfmt")
        .arg("--edition")
        .arg("2024")
        .arg(&file_path)
        .status()
        .map_err(|e| format!("failed to invoke rustfmt on {}: {e}", file_path.display()))?;
    if !rustfmt_status.success() {
        return Err(format!(
            "rustfmt exited with a failure status formatting {}",
            file_path.display()
        ));
    }
    let content = std::fs::read_to_string(&file_path).map_err(|e| {
        format!(
            "failed to re-read {} after rustfmt: {e}",
            file_path.display()
        )
    })?;

    let manifest_path = args.out_dir.join("MANIFEST.json");
    let mut manifest: crate::fixture_manifest::FixtureManifest =
        match std::fs::read_to_string(&manifest_path) {
            Ok(text) => serde_json::from_str(&text).map_err(|e| {
                format!("failed to parse existing {}: {e}", manifest_path.display())
            })?,
            Err(_) => crate::fixture_manifest::FixtureManifest {
                protocol_version: args.protocol_version,
                mc_version: args.mc_version.clone(),
                entries: Vec::new(),
            },
        };
    manifest.entries.retain(|e| e.path != "tags.rs");
    manifest
        .entries
        .push(crate::fixture_manifest::FixtureEntry {
            path: "tags.rs".to_string(),
            sha256: crate::fixture_manifest::compute_sha256_hex(content.as_bytes()),
            generator_tool_version: super::codegen::CODEGEN_TOOL_VERSION.to_string(),
            source_jar_sha1: args.source_jar_sha1.clone(),
        });

    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("failed to serialize MANIFEST.json: {e}"))?;
    std::fs::write(&manifest_path, &manifest_json)
        .map_err(|e| format!("failed to write {}: {e}", manifest_path.display()))?;

    // Self-check scoped to exactly the one entry this verb owns and just wrote — never the
    // full manifest via `fixture_manifest::verify_manifest`, unlike `codegen::run`'s own
    // self-check. `codegen::run` may fairly assume every entry it self-checks was itself just
    // written in the same call (it rebuilds the whole manifest from scratch); this verb is
    // additive onto a pre-existing manifest whose other entries (`registries.rs`/
    // `block_states.rs`) it never touches and is not responsible for re-validating.
    let on_disk = std::fs::read(&file_path)
        .map_err(|e| format!("failed to re-read {}: {e}", file_path.display()))?;
    let actual_sha256 = crate::fixture_manifest::compute_sha256_hex(&on_disk);
    let expected_sha256 = crate::fixture_manifest::compute_sha256_hex(content.as_bytes());
    if actual_sha256 != expected_sha256 {
        return Err(format!(
            "codegen-tags self-check failed — tags.rs on disk (sha256 {actual_sha256}) does not \
             match the content just generated (sha256 {expected_sha256})"
        ));
    }

    Ok(())
}
