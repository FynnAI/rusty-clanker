//! Tag-membership codegen — M1 registry-sync-fix follow-up
//! (`docs/research/mc-26.2/26-registry-sync-configuration.md` §5.3/§6, NET-D9/NET-D10).
//! Reads the vanilla datapack's own `data/minecraft/tags/**` JSON from a local
//! data-generator output directory (never committed raw — ASSET-D18(f)/NET-D9's carve-out
//! only covers the *derived*, code-generated Rust this module emits, mirroring `codegen.rs`'s
//! own already-established precedent for `registries.json`/`blocks.json`) plus the same
//! `--reports` `registries.json` `codegen.rs` already parses (for every static registry's own
//! numeric ids), and produces `crates/registries/generated/v<protocol_version>/tags.rs`.
//!
//! Round-3 real-client evidence (a genuine unmodified vanilla 26.2 client, crash report
//! `disconnect-2026-08-25_20.21.13-client.txt`) proved the earlier 5-registry minimal-set
//! cherry-pick incomplete: 8 remaining `enchantment` decode failures (`bane_of_arthropods`,
//! `channeling`, `impaling`, `power`, `punch`, `smite`, `soul_speed`, `wind_burst`) whose own
//! codecs reference tags in registries (`entity_type`, and further `block`/`item` tags) that
//! cherry-pick never sent. Scope widened accordingly: this module now discovers and emits
//! **every** registry directory actually present under the local tags tree that resolves to a
//! real, network-safe registry (real vanilla's own `networkSafeRegistries` union, §5.1) —
//! never a hand-picked subset — with every tag file it contains, not a curated root list. A
//! directory that cannot be resolved to any known registry (e.g. `villager_trade`,
//! `worldgen/structure` — real datapack content for a registry real vanilla never sends over
//! the wire, confirmed absent from `SYNCHRONIZED_REGISTRIES`) is named in `discover_registries`'s
//! own return value, never silently dropped.

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
/// `<tags_root>/data/minecraft/tags/<registry_dir>/` into a `TagTree`. `registry_dir` may
/// itself contain `/` (e.g. `"worldgen/biome"`) — `Path::join` splits on it correctly on every
/// platform this project targets, including Windows.
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
    /// A static registry (`block`, `item`, `entity_type`, ...) — numeric id = the real
    /// `registries.json` report's own `protocol_id` for that member name.
    Static(&'a RegistriesReport),
    /// A dynamic/registration-phase registry (`enchantment`, `dialog`, `timeline`, ...) —
    /// numeric id = the member's 0-based index position in `order`, which MUST stay identical
    /// to `crates/server/src/play/world.rs`'s own `SYNCHRONIZED_REGISTRIES` entry list for this
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

/// The complete `play::world::SYNCHRONIZED_REGISTRIES` order, hand-copied verbatim (29
/// registries, exact same names and exact same per-registry entry lists) — the single source
/// of truth for every dynamic-registry numeric id this module ever computes. MUST stay
/// identical to that constant; re-verify both together on any future change to either. Order
/// among registries carries no meaning here (§4.4); *within* one registry's own entry list,
/// order is load-bearing (it defines that registry's index-to-name mapping).
#[rustfmt::skip]
pub const SYNCHRONIZED_REGISTRIES_ORDER: &[(&str, &[&str])] = &[
    ("minecraft:dimension_type", &[
        "minecraft:overworld", "minecraft:overworld_caves", "minecraft:the_end", "minecraft:the_nether",
    ]),
    ("minecraft:worldgen/biome", &[
        "minecraft:plains", "minecraft:badlands", "minecraft:bamboo_jungle", "minecraft:basalt_deltas",
        "minecraft:beach", "minecraft:birch_forest", "minecraft:cherry_grove", "minecraft:cold_ocean",
        "minecraft:crimson_forest", "minecraft:dark_forest", "minecraft:deep_cold_ocean", "minecraft:deep_dark",
        "minecraft:deep_frozen_ocean", "minecraft:deep_lukewarm_ocean", "minecraft:deep_ocean", "minecraft:desert",
        "minecraft:dripstone_caves", "minecraft:end_barrens", "minecraft:end_highlands", "minecraft:end_midlands",
        "minecraft:eroded_badlands", "minecraft:flower_forest", "minecraft:forest", "minecraft:frozen_ocean",
        "minecraft:frozen_peaks", "minecraft:frozen_river", "minecraft:grove", "minecraft:ice_spikes",
        "minecraft:jagged_peaks", "minecraft:jungle", "minecraft:lukewarm_ocean", "minecraft:lush_caves",
        "minecraft:mangrove_swamp", "minecraft:meadow", "minecraft:mushroom_fields", "minecraft:nether_wastes",
        "minecraft:ocean", "minecraft:old_growth_birch_forest", "minecraft:old_growth_pine_taiga",
        "minecraft:old_growth_spruce_taiga", "minecraft:pale_garden", "minecraft:river", "minecraft:savanna",
        "minecraft:savanna_plateau", "minecraft:small_end_islands", "minecraft:snowy_beach", "minecraft:snowy_plains",
        "minecraft:snowy_slopes", "minecraft:snowy_taiga", "minecraft:soul_sand_valley", "minecraft:sparse_jungle",
        "minecraft:stony_peaks", "minecraft:stony_shore", "minecraft:sulfur_caves", "minecraft:sunflower_plains",
        "minecraft:swamp", "minecraft:taiga", "minecraft:the_end", "minecraft:the_void", "minecraft:warm_ocean",
        "minecraft:warped_forest", "minecraft:windswept_forest", "minecraft:windswept_gravelly_hills",
        "minecraft:windswept_hills", "minecraft:windswept_savanna", "minecraft:wooded_badlands",
    ]),
    ("minecraft:chat_type", &[
        "minecraft:chat", "minecraft:emote_command", "minecraft:msg_command_incoming",
        "minecraft:msg_command_outgoing", "minecraft:say_command", "minecraft:team_msg_command_incoming",
        "minecraft:team_msg_command_outgoing",
    ]),
    ("minecraft:trim_pattern", &[
        "minecraft:bolt", "minecraft:coast", "minecraft:dune", "minecraft:eye", "minecraft:flow",
        "minecraft:host", "minecraft:raiser", "minecraft:rib", "minecraft:sentry", "minecraft:shaper",
        "minecraft:silence", "minecraft:snout", "minecraft:spire", "minecraft:tide", "minecraft:vex",
        "minecraft:ward", "minecraft:wayfinder", "minecraft:wild",
    ]),
    ("minecraft:trim_material", &[
        "minecraft:amethyst", "minecraft:copper", "minecraft:diamond", "minecraft:emerald", "minecraft:gold",
        "minecraft:iron", "minecraft:lapis", "minecraft:netherite", "minecraft:quartz", "minecraft:redstone",
        "minecraft:resin",
    ]),
    ("minecraft:wolf_variant", &[
        "minecraft:ashen", "minecraft:black", "minecraft:chestnut", "minecraft:pale", "minecraft:rusty",
        "minecraft:snowy", "minecraft:spotted", "minecraft:striped", "minecraft:woods",
    ]),
    ("minecraft:wolf_sound_variant", &[
        "minecraft:angry", "minecraft:big", "minecraft:classic", "minecraft:cute", "minecraft:grumpy",
        "minecraft:puglin", "minecraft:sad",
    ]),
    ("minecraft:pig_variant", &["minecraft:cold", "minecraft:temperate", "minecraft:warm"]),
    ("minecraft:pig_sound_variant", &["minecraft:big", "minecraft:classic", "minecraft:mini"]),
    ("minecraft:frog_variant", &["minecraft:cold", "minecraft:temperate", "minecraft:warm"]),
    ("minecraft:cat_variant", &[
        "minecraft:all_black", "minecraft:black", "minecraft:british_shorthair", "minecraft:calico",
        "minecraft:jellie", "minecraft:persian", "minecraft:ragdoll", "minecraft:red", "minecraft:siamese",
        "minecraft:tabby", "minecraft:white",
    ]),
    ("minecraft:cat_sound_variant", &["minecraft:classic", "minecraft:royal"]),
    ("minecraft:cow_variant", &["minecraft:cold", "minecraft:temperate", "minecraft:warm"]),
    ("minecraft:cow_sound_variant", &["minecraft:classic", "minecraft:moody"]),
    ("minecraft:chicken_variant", &["minecraft:cold", "minecraft:temperate", "minecraft:warm"]),
    ("minecraft:chicken_sound_variant", &["minecraft:classic", "minecraft:picky"]),
    ("minecraft:zombie_nautilus_variant", &["minecraft:temperate", "minecraft:warm"]),
    ("minecraft:painting_variant", &[
        "minecraft:alban", "minecraft:aztec", "minecraft:aztec2", "minecraft:backyard", "minecraft:baroque",
        "minecraft:bomb", "minecraft:bouquet", "minecraft:burning_skull", "minecraft:bust", "minecraft:cavebird",
        "minecraft:changing", "minecraft:cotan", "minecraft:courbet", "minecraft:creebet", "minecraft:dennis",
        "minecraft:donkey_kong", "minecraft:earth", "minecraft:endboss", "minecraft:fern", "minecraft:fighters",
        "minecraft:finding", "minecraft:fire", "minecraft:graham", "minecraft:humble", "minecraft:kebab",
        "minecraft:lowmist", "minecraft:match", "minecraft:meditative", "minecraft:orb", "minecraft:owlemons",
        "minecraft:passage", "minecraft:pigscene", "minecraft:plant", "minecraft:pointer", "minecraft:pond",
        "minecraft:pool", "minecraft:prairie_ride", "minecraft:sea", "minecraft:skeleton",
        "minecraft:skull_and_roses", "minecraft:stage", "minecraft:sunflowers", "minecraft:sunset",
        "minecraft:tides", "minecraft:unpacked", "minecraft:void", "minecraft:wanderer", "minecraft:wasteland",
        "minecraft:water", "minecraft:wind", "minecraft:wither",
    ]),
    ("minecraft:damage_type", &[
        "minecraft:arrow", "minecraft:bad_respawn_point", "minecraft:cactus", "minecraft:campfire",
        "minecraft:cramming", "minecraft:dragon_breath", "minecraft:drown", "minecraft:dry_out",
        "minecraft:ender_pearl", "minecraft:explosion", "minecraft:fall", "minecraft:falling_anvil",
        "minecraft:falling_block", "minecraft:falling_stalactite", "minecraft:fireball", "minecraft:fireworks",
        "minecraft:fly_into_wall", "minecraft:freeze", "minecraft:generic", "minecraft:generic_kill",
        "minecraft:hot_floor", "minecraft:in_fire", "minecraft:in_wall", "minecraft:indirect_magic",
        "minecraft:lava", "minecraft:lightning_bolt", "minecraft:mace_smash", "minecraft:magic",
        "minecraft:mob_attack", "minecraft:mob_attack_no_aggro", "minecraft:mob_projectile", "minecraft:on_fire",
        "minecraft:out_of_world", "minecraft:outside_border", "minecraft:player_attack",
        "minecraft:player_explosion", "minecraft:sonic_boom", "minecraft:spear", "minecraft:spit",
        "minecraft:stalagmite", "minecraft:starve", "minecraft:sting", "minecraft:sulfur_cube_hot",
        "minecraft:sweet_berry_bush", "minecraft:thorns", "minecraft:thrown", "minecraft:trident",
        "minecraft:unattributed_fireball", "minecraft:wind_charge", "minecraft:wither", "minecraft:wither_skull",
    ]),
    ("minecraft:jukebox_song", &[
        "minecraft:11", "minecraft:13", "minecraft:5", "minecraft:blocks", "minecraft:bounce", "minecraft:cat",
        "minecraft:chirp", "minecraft:creator", "minecraft:creator_music_box", "minecraft:far",
        "minecraft:lava_chicken", "minecraft:mall", "minecraft:mellohi", "minecraft:otherside",
        "minecraft:pigstep", "minecraft:precipice", "minecraft:relic", "minecraft:stal", "minecraft:strad",
        "minecraft:tears", "minecraft:wait", "minecraft:ward",
    ]),
    ("minecraft:instrument", &[
        "minecraft:admire_goat_horn", "minecraft:call_goat_horn", "minecraft:dream_goat_horn",
        "minecraft:feel_goat_horn", "minecraft:ponder_goat_horn", "minecraft:seek_goat_horn",
        "minecraft:sing_goat_horn", "minecraft:yearn_goat_horn",
    ]),
    ("minecraft:banner_pattern", &[
        "minecraft:base", "minecraft:border", "minecraft:bricks", "minecraft:circle", "minecraft:creeper",
        "minecraft:cross", "minecraft:curly_border", "minecraft:diagonal_left", "minecraft:diagonal_right",
        "minecraft:diagonal_up_left", "minecraft:diagonal_up_right", "minecraft:flow", "minecraft:flower",
        "minecraft:globe", "minecraft:gradient", "minecraft:gradient_up", "minecraft:guster",
        "minecraft:half_horizontal", "minecraft:half_horizontal_bottom", "minecraft:half_vertical",
        "minecraft:half_vertical_right", "minecraft:mojang", "minecraft:piglin", "minecraft:rhombus",
        "minecraft:skull", "minecraft:small_stripes", "minecraft:square_bottom_left",
        "minecraft:square_bottom_right", "minecraft:square_top_left", "minecraft:square_top_right",
        "minecraft:straight_cross", "minecraft:stripe_bottom", "minecraft:stripe_center",
        "minecraft:stripe_downleft", "minecraft:stripe_downright", "minecraft:stripe_left",
        "minecraft:stripe_middle", "minecraft:stripe_right", "minecraft:stripe_top", "minecraft:triangle_bottom",
        "minecraft:triangle_top", "minecraft:triangles_bottom", "minecraft:triangles_top",
    ]),
    ("minecraft:enchantment", &[
        "minecraft:aqua_affinity", "minecraft:bane_of_arthropods", "minecraft:binding_curse",
        "minecraft:blast_protection", "minecraft:breach", "minecraft:channeling", "minecraft:density",
        "minecraft:depth_strider", "minecraft:efficiency", "minecraft:feather_falling", "minecraft:fire_aspect",
        "minecraft:fire_protection", "minecraft:flame", "minecraft:fortune", "minecraft:frost_walker",
        "minecraft:impaling", "minecraft:infinity", "minecraft:knockback", "minecraft:looting",
        "minecraft:loyalty", "minecraft:luck_of_the_sea", "minecraft:lunge", "minecraft:lure",
        "minecraft:mending", "minecraft:multishot", "minecraft:piercing", "minecraft:power",
        "minecraft:projectile_protection", "minecraft:protection", "minecraft:punch", "minecraft:quick_charge",
        "minecraft:respiration", "minecraft:riptide", "minecraft:sharpness", "minecraft:silk_touch",
        "minecraft:smite", "minecraft:soul_speed", "minecraft:sweeping_edge", "minecraft:swift_sneak",
        "minecraft:thorns", "minecraft:unbreaking", "minecraft:vanishing_curse", "minecraft:wind_burst",
    ]),
    ("minecraft:dialog", &[
        "minecraft:custom_options", "minecraft:quick_actions", "minecraft:server_links",
    ]),
    ("minecraft:timeline", &[
        "minecraft:day", "minecraft:early_game", "minecraft:moon", "minecraft:villager_schedule",
    ]),
    ("minecraft:world_clock", &["minecraft:overworld", "minecraft:the_end"]),
    ("minecraft:sulfur_cube_archetype", &[
        "minecraft:bouncy", "minecraft:explosive", "minecraft:fast_flat", "minecraft:fast_sliding",
        "minecraft:high_resistance", "minecraft:hot", "minecraft:light", "minecraft:regular",
        "minecraft:slow_bouncy", "minecraft:slow_flat", "minecraft:slow_sliding", "minecraft:sticky",
    ]),
    ("minecraft:test_environment", &["minecraft:default"]),
    ("minecraft:test_instance", &["minecraft:always_pass"]),
];

/// Which id space a discovered directory's own name resolved into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdSpaceKind {
    Static,
    Dynamic,
}

fn lookup_registry(candidate_id: &str, registries: &RegistriesReport) -> Option<IdSpaceKind> {
    if SYNCHRONIZED_REGISTRIES_ORDER
        .iter()
        .any(|(name, _)| *name == candidate_id)
    {
        Some(IdSpaceKind::Dynamic)
    } else if registries.contains_key(candidate_id) {
        Some(IdSpaceKind::Static)
    } else {
        None
    }
}

/// One tag directory under the local tags tree that resolved to a real, network-safe
/// registry.
pub struct DiscoveredRegistry {
    /// Directory path relative to `data/minecraft/tags/` (may contain `/`, e.g.
    /// `"worldgen/biome"`) — also this generated module's own name (sanitized).
    pub dir_path: String,
    /// Full resource-location registry id, as sent on the wire.
    pub registry_id: String,
    pub is_dynamic: bool,
}

/// Walks `<tags_root>/data/minecraft/tags/` top-down. At every directory, tests whether the
/// accumulated path-so-far names a real registry (present in `registries` — a static
/// registry — or in `SYNCHRONIZED_REGISTRIES_ORDER` — a dynamic one); if so, the *entire*
/// subtree below it is that registry's own tag-path space (never searched for further nested
/// registry boundaries — nested subdirectories from here on are tag paths, matching real
/// vanilla's own `block/mineable/pickaxe.json` -> tag `minecraft:mineable/pickaxe` shape). If
/// not, and the directory has its own subdirectories, recurses one level further (this is what
/// lets `worldgen/biome` resolve as a registry while `worldgen` itself, and `worldgen/structure`
/// alongside it, do not). A directory with no subdirectories that never matched anything is
/// named in the returned unresolved list — real datapack content whose registry a real vanilla
/// server never sends over the wire either (confirmed absent from both id spaces), never
/// silently dropped.
pub fn discover_registries(
    tags_root: &Path,
    registries: &RegistriesReport,
) -> Result<(Vec<DiscoveredRegistry>, Vec<String>), String> {
    let root = tags_root.join("data").join("minecraft").join("tags");
    let mut discovered = Vec::new();
    let mut unresolved = Vec::new();
    discover_node(&root, "", registries, &mut discovered, &mut unresolved)?;
    discovered.sort_by(|a, b| a.registry_id.cmp(&b.registry_id));
    unresolved.sort();
    Ok((discovered, unresolved))
}

fn discover_node(
    dir: &Path,
    prefix: &str,
    registries: &RegistriesReport,
    discovered: &mut Vec<DiscoveredRegistry>,
    unresolved: &mut Vec<String>,
) -> Result<(), String> {
    let read_dir = std::fs::read_dir(dir)
        .map_err(|e| format!("failed to read directory {}: {e}", dir.display()))?;
    let mut children: Vec<PathBuf> = Vec::new();
    for entry in read_dir {
        let entry = entry.map_err(|e| format!("failed to list {}: {e}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            children.push(path);
        }
    }
    children.sort();

    for child in children {
        let name = child
            .file_name()
            .expect("a directory entry always has a file name")
            .to_string_lossy()
            .into_owned();
        let candidate_path = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let candidate_id = format!("minecraft:{candidate_path}");

        match lookup_registry(&candidate_id, registries) {
            Some(kind) => discovered.push(DiscoveredRegistry {
                dir_path: candidate_path,
                registry_id: candidate_id,
                is_dynamic: kind == IdSpaceKind::Dynamic,
            }),
            None => {
                let has_subdirs = std::fs::read_dir(&child)
                    .map(|entries| entries.filter_map(|e| e.ok()).any(|e| e.path().is_dir()))
                    .unwrap_or(false);
                if has_subdirs {
                    discover_node(&child, &candidate_path, registries, discovered, unresolved)?;
                } else {
                    unresolved.push(candidate_path);
                }
            }
        }
    }
    Ok(())
}

/// Pure transform: `tags_root`'s own tag-file tree plus the real `registries` report in,
/// `(generated Rust source, unresolved directory paths)` out (or the first resolution/lookup
/// failure encountered, naming the offending tag/member). Every registry
/// `discover_registries` finds gets *every* tag file it contains emitted — never a curated
/// subset — sorted by registry id, then by tag path within each registry, for fully
/// deterministic output independent of filesystem iteration order.
pub fn generate_tags_rs(
    tags_root: &Path,
    registries: &RegistriesReport,
) -> Result<(String, Vec<String>), String> {
    let (discovered, unresolved) = discover_registries(tags_root, registries)?;

    let mut out = String::new();
    out.push_str("//! Generated by `xtask codegen-tags` — do not edit by hand, re-run instead.\n");
    out.push_str("//!\n");
    out.push_str(
        "//! NET-D9/NET-D10, docs/research/mc-26.2/26-registry-sync-configuration.md §5.3/§6:\n",
    );
    out.push_str(
        "//! the complete real vanilla tag set for every registry actually present under the\n",
    );
    out.push_str("//! local data generator's tags/** tree that resolves to a real, network-safe\n");
    out.push_str("//! registry (never a curated subset — round-3 real-client evidence proved a\n");
    out.push_str("//! hand-picked minimal set incomplete), resolved (including transitive #tag\n");
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

    let mut mod_infos: Vec<(String, String)> = Vec::with_capacity(discovered.len());

    for reg in &discovered {
        let tree = load_tag_tree(tags_root, &reg.dir_path)?;
        let space = if reg.is_dynamic {
            let order = SYNCHRONIZED_REGISTRIES_ORDER
                .iter()
                .find(|(name, _)| *name == reg.registry_id)
                .map(|(_, order)| *order)
                .expect("discover_node only ever marks is_dynamic=true for a registry it just matched against SYNCHRONIZED_REGISTRIES_ORDER");
            IdSpace::Dynamic(order)
        } else {
            IdSpace::Static(registries)
        };

        let mod_name = super::codegen::sanitize_mod_name(&reg.dir_path);
        out.push_str(&format!("pub mod {mod_name} {{\n"));
        out.push_str("    use super::TagTable;\n\n");

        let mut const_names: Vec<String> = Vec::with_capacity(tree.len());
        for tag_path in tree.keys() {
            let members = resolve_tag(tag_path, &tree)?;
            let mut ids: Vec<u32> = members
                .iter()
                .map(|member| numeric_id(&space, &reg.registry_id, member))
                .collect::<Result<_, _>>()?;
            ids.sort_unstable();
            ids.dedup();
            let ids_src: Vec<String> = ids.iter().map(|id| id.to_string()).collect();

            let const_name = super::codegen::sanitize_const_name(tag_path);
            if const_names.contains(&const_name) {
                return Err(format!(
                    "identifier collision in generated module {mod_name}: two different tag \
                     paths under {} both sanitize to {const_name} -- codegen cannot proceed",
                    reg.registry_id
                ));
            }
            out.push_str(&format!(
                "    pub const {const_name}: TagTable = TagTable {{ tag_id: \"minecraft:{tag_path}\", entries: &[{}] }};\n",
                ids_src.join(", ")
            ));
            const_names.push(const_name);
        }
        out.push_str(&format!(
            "\n    pub const TAGS: &[TagTable] = &[{}];\n",
            const_names.join(", ")
        ));
        out.push_str("}\n\n");

        mod_infos.push((mod_name, reg.registry_id.clone()));
    }

    out.push_str(
        "/// `(registry_id, tags)`, sorted by `registry_id` — order carries no wire-protocol\n",
    );
    out.push_str(
        "/// meaning (docs/research/mc-26.2/26-registry-sync-configuration.md §4.4), this is\n",
    );
    out.push_str("/// purely this table's own deterministic iteration order.\n");
    out.push_str("pub static REGISTRIES: &[(&str, &[TagTable])] = &[\n");
    for (mod_name, registry_id) in &mod_infos {
        out.push_str(&format!("    ({registry_id:?}, {mod_name}::TAGS),\n"));
    }
    out.push_str("];\n");

    Ok((out, unresolved))
}

pub struct TagsCodegenArgs {
    /// Directory containing `data/minecraft/tags/**` — typically a full (`--all`) local
    /// data-generator run's own `generated/` output, kept entirely outside this repository.
    pub tags_root: PathBuf,
    /// A prior `fetch-data` run's `generated/reports/` directory (`registries.json` is read
    /// from here — every static registry's own numeric ids).
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
/// `generate_tags_rs`, writes `tags.rs` under `args.out_dir`, prints every unresolved
/// directory path to stderr (never silent), then merges a `tags.rs` entry into that
/// directory's existing `MANIFEST.json` — additive: replaces a stale `tags.rs` entry from a
/// prior run if present, but never touches any other entry (`registries.rs`/`block_states.rs`
/// stay exactly as `codegen::run` last wrote them).
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

    let (content, unresolved) = generate_tags_rs(&args.tags_root, &registries)?;

    if unresolved.is_empty() {
        eprintln!("codegen-tags: every tag directory resolved to a network-safe registry");
    } else {
        eprintln!(
            "codegen-tags: {} tag director{} did not resolve to any known network-safe \
             registry (real datapack content real vanilla never sends over the wire either) \
             -- named, not silently skipped:",
            unresolved.len(),
            if unresolved.len() == 1 { "y" } else { "ies" }
        );
        for path in &unresolved {
            eprintln!("codegen-tags:   minecraft:{path}");
        }
    }

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
