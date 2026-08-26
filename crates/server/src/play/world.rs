//! The one hardcoded region and its 20 TPS tick loop -- this blueprint's own composition-
//! root wiring (M1-B05 blueprint Context, "The hardcoded region and its 20 TPS tick loop").
//! No `rc_scheduler::RegionManager` -- a single region that never splits or merges has no
//! use for its merge/split lifecycle; `RcExecutor::spawn_region` is called directly.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

use bevy_ecs::prelude::*;
use rc_chunk_storage::io_pool::ChunkNbtResolvers;
use rc_chunk_storage::lifecycle::ChunkLifecycleManager;
use rc_chunk_storage::superflat::SuperflatFiller;
use rc_chunk_storage::{
    AnvilDiskBackend, ChunkKeyTag, ChunkStorageBackend, CompressionScheme, PaletteThresholds,
};
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_messaging::{Address, RegionId, RegionMessageBus};
use rc_protocol::encode_payload;
use rc_registries::generated_v776::block_states::{
    self,
    default_state::{AIR, BEDROCK, DIRT, GRASS_BLOCK},
};
use rc_scheduler::chunk_ticket::{PlayerTicketId, TicketManager};
use rc_scheduler::pool::{RcWorkerPool, SystemTickWaiter, TickClock};
use rc_scheduler::{DomainGroup, RcExecutorBuilder};
use rc_transport_inproc::{InProcessTransport, InProcessTransportConfig};
use tokio::sync::oneshot;

use super::block_action::{
    ApplyOutcome, BLOCK_INTERACTION_RANGE_CREATIVE, ChunkIndex, DebugBlockInfo, PendingBlockAction,
    RejectReason, apply_block_action, debug_query_block, eye_position, target_position,
    to_storage_biome_id, to_storage_id, within_reach,
};
use super::connection::SPAWN_POSITION;
use super::packets::{AcknowledgeBlockChange, BlockUpdate, pack_position};
use super::registry_resolvers::McRegistryResolvers;
use super::{PlayerProfile, chunk, enter_play};
use crate::config::WorldConfig;
use crate::net::{ConnectionHandle, PlayerSession, PlayerSessionSink};

pub const HARDCODED_REGION_ID: RegionId = RegionId(1);

/// The full set of Configuration-phase synchronized registries the real composition root
/// (`main.rs`) advertises during registry-data sync, replacing the earlier 2-registry
/// (`dimension_type`, `worldgen/biome`) placeholder that let a real vanilla client through
/// only by accident of never needing anything else. A real 26.2 client's `finish_configuration`
/// requires every dynamic/datapack registry it knows about to have been sent at least an empty
/// `RegistryData` packet before it will proceed -- omitting one is a silent parity gap even
/// when nothing this server currently does would visibly break from the omission.
///
/// Registry set and every entry list sourced by cross-referencing two independently sanctioned
/// sources (never Mojang JSON, never decompiled Mojang source): (a) the pinned azalea rev's own
/// `azalea-registry/src/data.rs` `data_registry!` table, which enumerates exactly the
/// protocol-id-referenced registries a protocol-776 client is sent (`recipe` and
/// `enchantment_provider` also appear there but are not part of Configuration's own registry
/// sync -- recipes/enchantment-providers are never referenced by protocol id outside their own
/// dedicated packets, so they are deliberately excluded here); (b) minecraft.wiki's "Registry
/// data" configuration-packet page, whose own "List of synchronized registries" table matches
/// (a)'s registry set exactly (29 registries, byte-for-byte identical names) -- strong
/// cross-validation that (a) is both complete and current for this exact pinned version.
///
/// Every entry is sent `has_data=false` (`net::configuration_flow::run_configuration`'s own
/// unconditional default) -- the 1.20.5+ synchronized-registry semantics where an entry with no
/// inline payload tells the client "use your own already-loaded built-in definition for this
/// name," which is exactly correct for every entry below since every one of them is a stock
/// vanilla name a real client already ships. This project once special-cased
/// `minecraft:dimension_type` with hand-authored inline NBT instead (see git history) to work
/// around `azalea`'s own `RegistryHolder` never carrying built-in vanilla fallback data the way
/// a real client does; that path is deliberately not restored here even though it costs the
/// azalea-driven idle-stability smoke scenario (`crates/testing/paritybot`) its own ability to
/// resolve `dimension_type` -- hand-authoring a *complete* `dimension_type` payload ourselves
/// would need to reproduce vanilla's real `dimension_type` codec byte-for-byte from a
/// non-Mojang source, in a schema this exact pinned version appears to have recently
/// restructured (`environment_attribute`, a static registry `rc-registries`' own generated
/// `v776/registries.rs` already carries, replacing several legacy dimension_type fields); a
/// wrong hand-authored guess at that new schema risks silently reintroducing the very
/// real-client parse failure this table exists to fix, which `has_data=false` cannot ever do by
/// construction. See the M1 registry-sync completion report for the full trade-off writeup.
///
/// `minecraft:dimension_type`'s first entry stays `"minecraft:overworld"` and
/// `minecraft:worldgen/biome`'s first entry stays `"minecraft:plains"` -- `play::connection`'s
/// own `LoginPlay.dimension_type: 0` and `chunk.rs`'s own `PLACEHOLDER_BIOME_ID: u32 = 0` both
/// hardcode "index 0 of this table's own list resolves to that name," so reordering either
/// list is a wire-format break, not a cosmetic one.
pub const SYNCHRONIZED_REGISTRIES: &[(&str, &[&str])] = &[
    (
        "minecraft:dimension_type",
        &[
            "minecraft:overworld",
            "minecraft:overworld_caves",
            "minecraft:the_end",
            "minecraft:the_nether",
        ],
    ),
    (
        "minecraft:worldgen/biome",
        &[
            "minecraft:plains",
            "minecraft:badlands",
            "minecraft:bamboo_jungle",
            "minecraft:basalt_deltas",
            "minecraft:beach",
            "minecraft:birch_forest",
            "minecraft:cherry_grove",
            "minecraft:cold_ocean",
            "minecraft:crimson_forest",
            "minecraft:dark_forest",
            "minecraft:deep_cold_ocean",
            "minecraft:deep_dark",
            "minecraft:deep_frozen_ocean",
            "minecraft:deep_lukewarm_ocean",
            "minecraft:deep_ocean",
            "minecraft:desert",
            "minecraft:dripstone_caves",
            "minecraft:end_barrens",
            "minecraft:end_highlands",
            "minecraft:end_midlands",
            "minecraft:eroded_badlands",
            "minecraft:flower_forest",
            "minecraft:forest",
            "minecraft:frozen_ocean",
            "minecraft:frozen_peaks",
            "minecraft:frozen_river",
            "minecraft:grove",
            "minecraft:ice_spikes",
            "minecraft:jagged_peaks",
            "minecraft:jungle",
            "minecraft:lukewarm_ocean",
            "minecraft:lush_caves",
            "minecraft:mangrove_swamp",
            "minecraft:meadow",
            "minecraft:mushroom_fields",
            "minecraft:nether_wastes",
            "minecraft:ocean",
            "minecraft:old_growth_birch_forest",
            "minecraft:old_growth_pine_taiga",
            "minecraft:old_growth_spruce_taiga",
            "minecraft:pale_garden",
            "minecraft:river",
            "minecraft:savanna",
            "minecraft:savanna_plateau",
            "minecraft:small_end_islands",
            "minecraft:snowy_beach",
            "minecraft:snowy_plains",
            "minecraft:snowy_slopes",
            "minecraft:snowy_taiga",
            "minecraft:soul_sand_valley",
            "minecraft:sparse_jungle",
            "minecraft:stony_peaks",
            "minecraft:stony_shore",
            "minecraft:sulfur_caves",
            "minecraft:sunflower_plains",
            "minecraft:swamp",
            "minecraft:taiga",
            "minecraft:the_end",
            "minecraft:the_void",
            "minecraft:warm_ocean",
            "minecraft:warped_forest",
            "minecraft:windswept_forest",
            "minecraft:windswept_gravelly_hills",
            "minecraft:windswept_hills",
            "minecraft:windswept_savanna",
            "minecraft:wooded_badlands",
        ],
    ),
    (
        "minecraft:chat_type",
        &[
            "minecraft:chat",
            "minecraft:emote_command",
            "minecraft:msg_command_incoming",
            "minecraft:msg_command_outgoing",
            "minecraft:say_command",
            "minecraft:team_msg_command_incoming",
            "minecraft:team_msg_command_outgoing",
        ],
    ),
    (
        "minecraft:trim_pattern",
        &[
            "minecraft:bolt",
            "minecraft:coast",
            "minecraft:dune",
            "minecraft:eye",
            "minecraft:flow",
            "minecraft:host",
            "minecraft:raiser",
            "minecraft:rib",
            "minecraft:sentry",
            "minecraft:shaper",
            "minecraft:silence",
            "minecraft:snout",
            "minecraft:spire",
            "minecraft:tide",
            "minecraft:vex",
            "minecraft:ward",
            "minecraft:wayfinder",
            "minecraft:wild",
        ],
    ),
    (
        "minecraft:trim_material",
        &[
            "minecraft:amethyst",
            "minecraft:copper",
            "minecraft:diamond",
            "minecraft:emerald",
            "minecraft:gold",
            "minecraft:iron",
            "minecraft:lapis",
            "minecraft:netherite",
            "minecraft:quartz",
            "minecraft:redstone",
            "minecraft:resin",
        ],
    ),
    (
        "minecraft:wolf_variant",
        &[
            "minecraft:ashen",
            "minecraft:black",
            "minecraft:chestnut",
            "minecraft:pale",
            "minecraft:rusty",
            "minecraft:snowy",
            "minecraft:spotted",
            "minecraft:striped",
            "minecraft:woods",
        ],
    ),
    (
        "minecraft:wolf_sound_variant",
        &[
            "minecraft:angry",
            "minecraft:big",
            "minecraft:classic",
            "minecraft:cute",
            "minecraft:grumpy",
            "minecraft:puglin",
            "minecraft:sad",
        ],
    ),
    (
        "minecraft:pig_variant",
        &["minecraft:cold", "minecraft:temperate", "minecraft:warm"],
    ),
    (
        "minecraft:pig_sound_variant",
        &["minecraft:big", "minecraft:classic", "minecraft:mini"],
    ),
    (
        "minecraft:frog_variant",
        &["minecraft:cold", "minecraft:temperate", "minecraft:warm"],
    ),
    (
        "minecraft:cat_variant",
        &[
            "minecraft:all_black",
            "minecraft:black",
            "minecraft:british_shorthair",
            "minecraft:calico",
            "minecraft:jellie",
            "minecraft:persian",
            "minecraft:ragdoll",
            "minecraft:red",
            "minecraft:siamese",
            "minecraft:tabby",
            "minecraft:white",
        ],
    ),
    (
        "minecraft:cat_sound_variant",
        &["minecraft:classic", "minecraft:royal"],
    ),
    (
        "minecraft:cow_variant",
        &["minecraft:cold", "minecraft:temperate", "minecraft:warm"],
    ),
    (
        "minecraft:cow_sound_variant",
        &["minecraft:classic", "minecraft:moody"],
    ),
    (
        "minecraft:chicken_variant",
        &["minecraft:cold", "minecraft:temperate", "minecraft:warm"],
    ),
    (
        "minecraft:chicken_sound_variant",
        &["minecraft:classic", "minecraft:picky"],
    ),
    (
        "minecraft:zombie_nautilus_variant",
        &["minecraft:temperate", "minecraft:warm"],
    ),
    (
        "minecraft:painting_variant",
        &[
            "minecraft:alban",
            "minecraft:aztec",
            "minecraft:aztec2",
            "minecraft:backyard",
            "minecraft:baroque",
            "minecraft:bomb",
            "minecraft:bouquet",
            "minecraft:burning_skull",
            "minecraft:bust",
            "minecraft:cavebird",
            "minecraft:changing",
            "minecraft:cotan",
            "minecraft:courbet",
            "minecraft:creebet",
            "minecraft:dennis",
            "minecraft:donkey_kong",
            "minecraft:earth",
            "minecraft:endboss",
            "minecraft:fern",
            "minecraft:fighters",
            "minecraft:finding",
            "minecraft:fire",
            "minecraft:graham",
            "minecraft:humble",
            "minecraft:kebab",
            "minecraft:lowmist",
            "minecraft:match",
            "minecraft:meditative",
            "minecraft:orb",
            "minecraft:owlemons",
            "minecraft:passage",
            "minecraft:pigscene",
            "minecraft:plant",
            "minecraft:pointer",
            "minecraft:pond",
            "minecraft:pool",
            "minecraft:prairie_ride",
            "minecraft:sea",
            "minecraft:skeleton",
            "minecraft:skull_and_roses",
            "minecraft:stage",
            "minecraft:sunflowers",
            "minecraft:sunset",
            "minecraft:tides",
            "minecraft:unpacked",
            "minecraft:void",
            "minecraft:wanderer",
            "minecraft:wasteland",
            "minecraft:water",
            "minecraft:wind",
            "minecraft:wither",
        ],
    ),
    (
        "minecraft:damage_type",
        &[
            "minecraft:arrow",
            "minecraft:bad_respawn_point",
            "minecraft:cactus",
            "minecraft:campfire",
            "minecraft:cramming",
            "minecraft:dragon_breath",
            "minecraft:drown",
            "minecraft:dry_out",
            "minecraft:ender_pearl",
            "minecraft:explosion",
            "minecraft:fall",
            "minecraft:falling_anvil",
            "minecraft:falling_block",
            "minecraft:falling_stalactite",
            "minecraft:fireball",
            "minecraft:fireworks",
            "minecraft:fly_into_wall",
            "minecraft:freeze",
            "minecraft:generic",
            "minecraft:generic_kill",
            "minecraft:hot_floor",
            "minecraft:in_fire",
            "minecraft:in_wall",
            "minecraft:indirect_magic",
            "minecraft:lava",
            "minecraft:lightning_bolt",
            "minecraft:mace_smash",
            "minecraft:magic",
            "minecraft:mob_attack",
            "minecraft:mob_attack_no_aggro",
            "minecraft:mob_projectile",
            "minecraft:on_fire",
            "minecraft:out_of_world",
            "minecraft:outside_border",
            "minecraft:player_attack",
            "minecraft:player_explosion",
            "minecraft:sonic_boom",
            "minecraft:spear",
            "minecraft:spit",
            "minecraft:stalagmite",
            "minecraft:starve",
            "minecraft:sting",
            "minecraft:sulfur_cube_hot",
            "minecraft:sweet_berry_bush",
            "minecraft:thorns",
            "minecraft:thrown",
            "minecraft:trident",
            "minecraft:unattributed_fireball",
            "minecraft:wind_charge",
            "minecraft:wither",
            "minecraft:wither_skull",
        ],
    ),
    (
        "minecraft:jukebox_song",
        &[
            "minecraft:11",
            "minecraft:13",
            "minecraft:5",
            "minecraft:blocks",
            "minecraft:bounce",
            "minecraft:cat",
            "minecraft:chirp",
            "minecraft:creator",
            "minecraft:creator_music_box",
            "minecraft:far",
            "minecraft:lava_chicken",
            "minecraft:mall",
            "minecraft:mellohi",
            "minecraft:otherside",
            "minecraft:pigstep",
            "minecraft:precipice",
            "minecraft:relic",
            "minecraft:stal",
            "minecraft:strad",
            "minecraft:tears",
            "minecraft:wait",
            "minecraft:ward",
        ],
    ),
    (
        "minecraft:instrument",
        &[
            "minecraft:admire_goat_horn",
            "minecraft:call_goat_horn",
            "minecraft:dream_goat_horn",
            "minecraft:feel_goat_horn",
            "minecraft:ponder_goat_horn",
            "minecraft:seek_goat_horn",
            "minecraft:sing_goat_horn",
            "minecraft:yearn_goat_horn",
        ],
    ),
    (
        "minecraft:banner_pattern",
        &[
            "minecraft:base",
            "minecraft:border",
            "minecraft:bricks",
            "minecraft:circle",
            "minecraft:creeper",
            "minecraft:cross",
            "minecraft:curly_border",
            "minecraft:diagonal_left",
            "minecraft:diagonal_right",
            "minecraft:diagonal_up_left",
            "minecraft:diagonal_up_right",
            "minecraft:flow",
            "minecraft:flower",
            "minecraft:globe",
            "minecraft:gradient",
            "minecraft:gradient_up",
            "minecraft:guster",
            "minecraft:half_horizontal",
            "minecraft:half_horizontal_bottom",
            "minecraft:half_vertical",
            "minecraft:half_vertical_right",
            "minecraft:mojang",
            "minecraft:piglin",
            "minecraft:rhombus",
            "minecraft:skull",
            "minecraft:small_stripes",
            "minecraft:square_bottom_left",
            "minecraft:square_bottom_right",
            "minecraft:square_top_left",
            "minecraft:square_top_right",
            "minecraft:straight_cross",
            "minecraft:stripe_bottom",
            "minecraft:stripe_center",
            "minecraft:stripe_downleft",
            "minecraft:stripe_downright",
            "minecraft:stripe_left",
            "minecraft:stripe_middle",
            "minecraft:stripe_right",
            "minecraft:stripe_top",
            "minecraft:triangle_bottom",
            "minecraft:triangle_top",
            "minecraft:triangles_bottom",
            "minecraft:triangles_top",
        ],
    ),
    (
        "minecraft:enchantment",
        &[
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
        ],
    ),
    (
        "minecraft:dialog",
        &[
            "minecraft:custom_options",
            "minecraft:quick_actions",
            "minecraft:server_links",
        ],
    ),
    (
        "minecraft:timeline",
        &[
            "minecraft:day",
            "minecraft:early_game",
            "minecraft:moon",
            "minecraft:villager_schedule",
        ],
    ),
    (
        "minecraft:world_clock",
        &["minecraft:overworld", "minecraft:the_end"],
    ),
    (
        "minecraft:sulfur_cube_archetype",
        &[
            "minecraft:bouncy",
            "minecraft:explosive",
            "minecraft:fast_flat",
            "minecraft:fast_sliding",
            "minecraft:high_resistance",
            "minecraft:hot",
            "minecraft:light",
            "minecraft:regular",
            "minecraft:slow_bouncy",
            "minecraft:slow_flat",
            "minecraft:slow_sliding",
            "minecraft:sticky",
        ],
    ),
    ("minecraft:test_environment", &["minecraft:default"]),
    ("minecraft:test_instance", &["minecraft:always_pass"]),
];

#[derive(Component)]
pub struct PlayerMarker {
    pub network_entity_id: i32,
    pub username: String,
    /// New (M2-B07 Context: "The M1-B05 interest/broadcast seam does not exist -- resolved
    /// here") -- lets the tick thread reach every connected player's socket directly, for
    /// the block-update broadcast every currently-connected player is, by this world's own
    /// fixed shape, interested in.
    pub connection: ConnectionHandle,
}

pub struct PendingJoin {
    pub network_entity_id: i32,
    pub username: String,
    /// New -- carried from `enter_play`'s Tokio task across the same channel boundary
    /// `network_entity_id`/`username` already cross.
    pub connection: ConnectionHandle,
}

/// The region-build-time bootstrap `RcExecutorBuilder::new` requires (a plain `fn`
/// pointer, not a closure -- M0-B05's own required shape). M2-B05 replaces M2-B07's own
/// static, 121-chunk-at-build-time bootstrap with real, ticket-driven, storage-backed
/// chunk streaming (this blueprint's own Goal) -- no chunk entity is ever spawned here
/// any more; `ChunkLifecycleManager::pre_tick` spawns them on demand as `TicketManager`'s
/// churn requests them. Only `ChunkIndex` (M2-B07's own `ChunkKey -> Entity` directory,
/// `block_action.rs`) needs a value present before the very first tick's block-action
/// processing step could otherwise read it -- kept empty here; the tick loop below
/// refreshes it every round from `region.world`'s own current chunk entities, immediately
/// after `lifecycle.pre_tick` runs, so `apply_block_action`/`debug_query_block` (M2-B07)
/// always see this tick's real, current residency set without either blueprint needing to
/// know about the other's own internals (`rc-chunk-storage`'s `ChunkLifecycleManager`
/// never depends on any `rusty-clanker-server` type, Context's own dependency-graph note,
/// generalized to this composition-root/M2-B07 boundary too).
fn bootstrap_region(world: &mut World) {
    world.insert_resource(ChunkIndex::default());
}

/// Every raw id and threshold `SuperflatFiller`/`ChunkNbtResolvers` need, converted once
/// at composition-root time from `rc_registries::generated_v776`'s own raw `u32` ids into
/// `rc-chunk-storage`'s own distinct id newtypes (M2-B01's own reserved seam,
/// `block_action.rs`'s `to_storage_id`/`to_storage_biome_id`, reused unmodified) --
/// M2-B05's own restatement of `M1-B05`'s already-merged, byte-verified superflat layer
/// table (M2-B05 blueprint Context: "Superflat filler").
fn superflat_filler() -> SuperflatFiller {
    let block_direct_bits = rc_chunk_storage::ceil_log2(block_states::BLOCK_STATE_COUNT) as u16;
    SuperflatFiller {
        air: to_storage_id(AIR.0),
        bedrock: to_storage_id(BEDROCK.0),
        dirt: to_storage_id(DIRT.0),
        grass: to_storage_id(GRASS_BLOCK.0),
        biome: to_storage_biome_id(chunk::PLACEHOLDER_BIOME_ID),
        block_thresholds: PaletteThresholds::blocks(block_direct_bits),
        // No generated `worldgen_biome` registry table exists yet (`chunk.rs`'s own
        // confirmed deviation, restated in `block_action.rs`'s and
        // `registry_resolvers.rs`'s own module doc comments) -- mirrors `chunk.rs`'s own
        // private `PLACEHOLDER_BIOME_REGISTRY_COUNT` (64) rather than importing it (not
        // `pub`); inconsequential, since this filler's own biome column is always
        // single-valued.
        biome_thresholds: PaletteThresholds::biomes(rc_chunk_storage::ceil_log2(64) as u16),
    }
}

/// Owns the one hardcoded region's tick loop (its own dedicated OS thread, ARCH-D21) and a
/// network-entity-id counter, independent of `rc_core::RcEntityIdAllocator` (Context --
/// vanilla's own wire `entity_id` is a separate, small `i32` space). `Clone`, cheap (an
/// `Arc`-backed sender handle).
#[derive(Clone)]
pub struct HardcodedWorld {
    join_tx: tokio::sync::mpsc::UnboundedSender<PendingJoin>,
    /// New (M2-B07): enqueued by `connection.rs`'s inbound dispatch, drained once per tick
    /// at this region's own Stage-3-equivalent manual step (Context, "Which pipeline
    /// stage").
    block_action_tx: tokio::sync::mpsc::UnboundedSender<PendingBlockAction>,
    /// New (M2-B07), test/diagnostic only -- `debug_query_block`'s own doc comment.
    query_tx:
        tokio::sync::mpsc::UnboundedSender<(BlockPos, oneshot::Sender<Option<DebugBlockInfo>>)>,
    next_network_entity_id: Arc<AtomicI32>,
    /// New (M2-B05): signals the region thread to stop after finishing its current round
    /// (`shutdown`'s own doc comment).
    shutdown_flag: Arc<AtomicBool>,
    /// New (M2-B05): the region thread's own `JoinHandle`, taken and joined by `shutdown`.
    thread_handle: Arc<Mutex<Option<std::thread::JoinHandle<()>>>>,
}

impl HardcodedWorld {
    /// Backward-compatible zero-argument constructor (M1-B05's own original signature,
    /// still relied on by every M1/M2-B06/M2-B07 acceptance test this blueprint's own
    /// Constraints forbid editing). M2-B05 implementation note (a forced, necessary
    /// deviation from this blueprint's own literal `HardcodedWorld::new(config:
    /// WorldConfig)` Deliverables signature, recorded here and in the implementation
    /// changeset's commit body): six already-committed test files across M1-B05/M2-B06/
    /// M2-B07 call `HardcodedWorld::new()` with no arguments; changing that signature
    /// would be a breaking edit to test files this blueprint's own process forbids
    /// touching. Resolved by keeping `new()` as the zero-argument form -- `with_config`,
    /// below, is the real composition-root entry point this blueprint's own Deliverables
    /// describe, used by `main.rs`. `new()` delegates to `with_config` with every
    /// `WorldConfig` default except `world_dir`, which is instead a fresh, uniquely-named
    /// directory under `std::env::temp_dir()` each call -- every one of those six test
    /// files runs as its own OS process (Cargo integration-test convention) and some run
    /// concurrently (`cargo nextest`'s own default execution model); sharing a single
    /// relative `"world"` directory across them would make every process but the first to
    /// call `AnvilDiskBackend::open` observe `StorageError::WorldAlreadyOpen`
    /// (`AnvilDiskBackend`'s own real, advisory OS-level `session.lock`, M2-B03) --
    /// non-deterministically, depending on process scheduling. None of those six test
    /// files ever inspects on-disk world storage, so a private, disposable directory per
    /// call changes nothing they observe.
    pub fn new() -> Self {
        Self::with_config(WorldConfig {
            world_dir: unique_temp_world_dir(),
            ..WorldConfig::default()
        })
    }

    /// The real composition-root constructor (M2-B05 blueprint Deliverables): opens
    /// `config.world_dir` as a real `AnvilDiskBackend` (B03), wires a `TicketManager`
    /// (`rc-scheduler`) and a `ChunkLifecycleManager` (`rc-chunk-storage`) into the tick
    /// loop around `M1-B05`'s/`M2-B07`'s own existing join-drain/block-action/tick_region
    /// steps, and registers the Stage-9 snapshot system before building the executor.
    /// Spawns the tick-loop thread and returns a handle; the thread runs until `shutdown`
    /// is called.
    pub fn with_config(config: WorldConfig) -> Self {
        let (join_tx, mut join_rx) = tokio::sync::mpsc::unbounded_channel::<PendingJoin>();
        let (block_action_tx, mut block_action_rx) =
            tokio::sync::mpsc::unbounded_channel::<PendingBlockAction>();
        let (query_tx, mut query_rx) = tokio::sync::mpsc::unbounded_channel::<(
            BlockPos,
            oneshot::Sender<Option<DebugBlockInfo>>,
        )>();
        let shutdown_flag = Arc::new(AtomicBool::new(false));

        let thread_shutdown_flag = Arc::clone(&shutdown_flag);
        let handle = std::thread::spawn(move || {
            let backend: Arc<dyn ChunkStorageBackend> = Arc::new(
                AnvilDiskBackend::open(config.world_dir.clone(), CompressionScheme::Zlib).expect(
                    "the world directory must be openable (M2-B05 Deliverables: a \
                     real, checkable failure this composition root never silently \
                     swallows)",
                ),
            );
            let resolvers = Arc::new(ChunkNbtResolvers {
                block_names: Box::new(McRegistryResolvers),
                biome_names: Box::new(McRegistryResolvers),
                block_thresholds: PaletteThresholds::blocks(rc_chunk_storage::ceil_log2(
                    block_states::BLOCK_STATE_COUNT,
                ) as u16),
                biome_thresholds: PaletteThresholds::biomes(rc_chunk_storage::ceil_log2(64) as u16),
            });
            let mut ticket_manager = TicketManager::new();
            let mut lifecycle = ChunkLifecycleManager::new(
                Arc::clone(&backend),
                DimensionId::OVERWORLD,
                superflat_filler(),
                resolvers,
                config.save_interval_ticks(),
                4096,
            );
            // M2 integration addition: opt-in `--save-event-log` wiring (Context,
            // `SaveEventSink`'s own doc comment) -- absent for every ordinary run
            // (`WorldConfig::save_event_log` defaults to `None`), so this changes
            // nothing observable unless the composition root was explicitly asked
            // for a save-event log. `HARDCODED_REGION_ID.0` (an integer) is this
            // manager's own stable `region_id` label for the log's whole lifetime --
            // M2 stays single-region (Context, "M2 stays inside M1-B05's single
            // HARDCODED_REGION_ID"), so there is exactly one label to pick.
            if let Some(log_path) = &config.save_event_log
                && let Err(err) =
                    lifecycle.install_save_event_log(HARDCODED_REGION_ID.0.to_string(), log_path)
            {
                tracing::error!(
                    path = %log_path.display(),
                    error = %err,
                    "failed to open --save-event-log; continuing without save-event logging"
                );
            }

            let mut builder = RcExecutorBuilder::new(bootstrap_region);
            builder.register_system(
                DomainGroup::ChunkSerialize,
                rc_chunk_storage::lifecycle::snapshot_system_factory(),
                vec![],
            );
            let executor = builder.build().expect(
                "the Stage-9 snapshot system never violates ARCH-D8's structural-write check",
            );
            let mut region = executor.spawn_region(HARDCODED_REGION_ID);
            lifecycle.install_resources(&mut region.world);

            let transport = InProcessTransport::new(InProcessTransportConfig::default());
            transport.register_region(HARDCODED_REGION_ID);
            let pool = RcWorkerPool::new(4);
            let mut clock = TickClock::<SystemTickWaiter>::new();
            // M2-B05 implementation note (a forced, necessary addition, recorded here and
            // in the implementation changeset's commit body): persists across loop
            // iterations -- a `Break`/`Place` action whose target chunk has not yet
            // finished its (now async, ticket-driven) load is carried over to a later
            // tick instead of being dropped as a `ChunkIndex` miss. M2-B07's own static,
            // build-time 121-chunk bootstrap (this blueprint's own Goal replaces it with
            // real, ticket-driven streaming) made every reachable chunk resident from
            // tick 0 onward, so an action processed on the very same tick as its actor's
            // own join always found its target chunk already loaded; the real async load
            // this blueprint introduces needs at least one further tick to complete, and
            // under concurrent-test scheduling load a join and its very next action can
            // legitimately land on the very same tick. `Ignored`-kind and out-of-reach
            // actions never depend on chunk residency and are still always resolved (and
            // acked) the same tick they arrive.
            let mut carried_block_actions: Vec<PendingBlockAction> = Vec::new();

            loop {
                if thread_shutdown_flag.load(Ordering::Relaxed) {
                    lifecycle.shutdown(&region.world);
                    return;
                }

                while let Ok(join) = join_rx.try_recv() {
                    region.world.spawn(PlayerMarker {
                        network_entity_id: join.network_entity_id,
                        username: join.username,
                        connection: join.connection,
                    });
                    let spawn_chunk = SPAWN_POSITION.chunk_key(DimensionId::OVERWORLD);
                    ticket_manager.register_player(
                        PlayerTicketId(join.network_entity_id),
                        spawn_chunk,
                        config.simulation_distance_chunks,
                    );
                }

                // M2-B05's own load/unload churn, immediately before `tick_region`
                // (Context: "restate which stage and sync point", `M1-B05`'s own
                // established Stage-1 pre-tick-sync-point precedent).
                let churn = ticket_manager.step();
                lifecycle.pre_tick(&mut region.world, &churn.needs_load, &churn.needs_unload);

                // Refreshes M2-B07's own `ChunkIndex` directory from this tick's real,
                // current chunk residency -- the composition-root bridge between
                // `ChunkLifecycleManager` (which knows nothing of `ChunkIndex`, a
                // `rusty-clanker-server`-only type) and `block_action.rs`'s own lookups
                // (`bootstrap_region`'s own doc comment).
                let mut chunk_index = ChunkIndex::default();
                let mut chunk_query = region.world.query::<(&ChunkKeyTag, Entity)>();
                for (tag, entity) in chunk_query.iter(&region.world) {
                    chunk_index.0.insert(tag.0, entity);
                }
                region.world.insert_resource(chunk_index);

                // M2-B07's own Stage-3-equivalent manual step (Context, "Which pipeline
                // stage"): drain every block action queued since the previous tick,
                // stable-sort by ascending `network_entity_id` (MECH-D4's "deterministic
                // merge by ascending player id"), reach-validate (Context, "Where this
                // check runs, precisely" -- deliberately not `apply_block_action`'s own
                // concern), then apply/route and respond -- entirely before
                // `executor.tick_region` runs this tick's own formally-numbered pipeline.
                let mut pending: Vec<PendingBlockAction> =
                    std::mem::take(&mut carried_block_actions);
                while let Ok(action) = block_action_rx.try_recv() {
                    pending.push(action);
                }
                pending.sort_by_key(|action| action.network_entity_id);

                let mut bus = RegionMessageBus::new();
                // M2 stays single-region (Context: "M2 stays inside M1-B05's single
                // HARDCODED_REGION_ID" -- no per-chunk `ChunkKey -> RegionId` directory
                // exists yet, ARCH-D24's own deferred item) -- every chunk key is
                // trivially local by construction. M2-B05 implementation note (a forced,
                // necessary deviation from M2-B07's own committed `local_chunk_keys`
                // check, recorded here and in the implementation changeset's commit
                // body): that check compared against the *static* 121-chunk bootstrap set
                // this blueprint just removed (`bootstrap_region`'s own doc comment);
                // dynamic, ticket-driven residency has no equivalent fixed set to check
                // against, and a chunk that is local but merely not yet finished loading
                // must not be misreported as belonging to a different (nonexistent)
                // region -- unconditional locality is both simpler and more correct at
                // M2's own single-region scope.
                let resolve_owner = |_key: ChunkKey| Address::Region(HARDCODED_REGION_ID);

                for action in pending {
                    let target = target_position(&action.kind);
                    let out_of_reach = target.is_some_and(|target| {
                        !within_reach(
                            eye_position(SPAWN_POSITION),
                            target,
                            BLOCK_INTERACTION_RANGE_CREATIVE,
                        )
                    });
                    // Only a reach-validated `Break`/`Place` action needs its target
                    // chunk resident before it can be applied -- an `Ignored` action
                    // (`target` is `None`) or an out-of-reach one is fully resolved
                    // without ever touching chunk data (`apply_block_action`'s own
                    // Deliverables: reach is deliberately not its own concern).
                    if let Some(target) = target
                        && !out_of_reach
                        && !lifecycle.is_resident(target.chunk_key(DimensionId::OVERWORLD))
                    {
                        carried_block_actions.push(action);
                        continue;
                    }

                    let outcome = if out_of_reach {
                        ApplyOutcome::Rejected {
                            pos: target
                                .expect("out_of_reach is only ever true when target is Some"),
                            reason: RejectReason::OutOfReach,
                            current_state: None,
                        }
                    } else if target.is_none() {
                        ApplyOutcome::NoOp
                    } else {
                        apply_block_action(
                            &mut region.world,
                            DimensionId::OVERWORLD,
                            &action,
                            &resolve_owner,
                            Address::Region(HARDCODED_REGION_ID),
                            &mut bus,
                        )
                    };
                    respond_to_action(&region.world, &action, outcome);
                }
                region.message_state.merge(bus);

                while let Ok((pos, reply)) = query_rx.try_recv() {
                    let _ = reply.send(debug_query_block(
                        &region.world,
                        DimensionId::OVERWORLD,
                        pos,
                    ));
                }

                executor.tick_region(&mut region, &pool, &transport);
                lifecycle.post_tick();
                clock.await_next_tick();
            }
        });

        Self {
            join_tx,
            block_action_tx,
            query_tx,
            next_network_entity_id: Arc::new(AtomicI32::new(1)),
            shutdown_flag,
            thread_handle: Arc::new(Mutex::new(Some(handle))),
        }
    }

    /// Allocates the next network-facing entity id (starts at `1`, monotonic, thread-safe).
    pub fn alloc_network_entity_id(&self) -> i32 {
        self.next_network_entity_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Enqueues a `PlayerMarker` spawn, applied at the start of the region's next tick
    /// (Context's join-queue). Never blocks (`UnboundedSender::send` never blocks).
    pub fn queue_join(&self, join: PendingJoin) {
        self.join_tx
            .send(join)
            .expect("the hardcoded region's tick-loop thread outlives every connection");
    }

    /// Signals the region thread to stop after finishing its current tick, run
    /// `ChunkLifecycleManager::shutdown` (WORLD-D25's flush barrier), and exit; blocks the
    /// calling thread until the region thread has actually joined. Never call this
    /// directly from an async context without `tokio::task::spawn_blocking` -- this call
    /// blocks synchronously.
    pub fn shutdown(&self) {
        self.shutdown_flag.store(true, Ordering::Relaxed);
        if let Some(handle) = self
            .thread_handle
            .lock()
            .expect("the region thread never panics while holding this lock")
            .take()
        {
            let _ = handle.join();
        }
    }

    /// New. Enqueues a decoded block action, applied at the start of this region's next
    /// tick's Stage-3-equivalent step (Context). Never blocks.
    pub fn queue_block_action(&self, action: PendingBlockAction) {
        self.block_action_tx
            .send(action)
            .expect("the hardcoded region's tick-loop thread outlives every connection");
    }

    /// New, test/diagnostic only (Context, `debug_query_block`'s own doc comment). Awaits
    /// this tick's or the next tick's debug-query drain step, whichever comes first after
    /// the call.
    pub async fn debug_query_block(&self, pos: BlockPos) -> Option<DebugBlockInfo> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.query_tx
            .send((pos, reply_tx))
            .expect("the hardcoded region's tick-loop thread outlives every connection");
        reply_rx.await.expect(
            "the hardcoded region's tick-loop thread always replies before dropping the sender",
        )
    }
}

/// `apply_block_action`'s response side (Context, Implementation step 8's own
/// `respond_to_action` algorithm): always one `Acknowledge Block Change` to the acting
/// connection first (MECH-D63 -- unconditional, whether the action succeeded or was
/// rejected), then, depending on `outcome`, either a broadcast `Block Update` to every
/// currently-connected player (including the actor itself -- Context explains why that is
/// a deliberate, harmless superset of vanilla's own actor-excluded broadcast) or a
/// corrective `Block Update` to the actor alone. Iterates `world`'s entities directly
/// (`EntityRef::get`) rather than `World::query` so this function can take `&World`
/// (matching the tick loop's own `&region.world` call site) instead of `&mut World`.
fn respond_to_action(world: &World, action: &PendingBlockAction, outcome: ApplyOutcome) {
    let _ = action
        .connection
        .try_send_payload(encode_payload(&AcknowledgeBlockChange {
            sequence: action.sequence,
        }));

    match outcome {
        ApplyOutcome::Applied { pos, new_state }
        | ApplyOutcome::RoutedCrossRegion { pos, new_state } => {
            let payload = encode_payload(&BlockUpdate {
                location: pack_position(pos),
                block_state_id: new_state as i32,
            });
            for entity_ref in world.iter_entities() {
                if let Some(marker) = entity_ref.get::<PlayerMarker>() {
                    let _ = marker.connection.try_send_payload(payload.clone());
                }
            }
        }
        ApplyOutcome::Rejected {
            pos,
            current_state: Some(current),
            ..
        } => {
            let payload = encode_payload(&BlockUpdate {
                location: pack_position(pos),
                block_state_id: current as i32,
            });
            let _ = action.connection.try_send_payload(payload);
        }
        ApplyOutcome::Rejected {
            current_state: None,
            ..
        }
        | ApplyOutcome::NoOp => {}
    }
}

impl Default for HardcodedWorld {
    fn default() -> Self {
        Self::new()
    }
}

/// M1-B04's real Configuration->Play hand-off (Context, "Assumed hand-off from the
/// connection driver") -- translates `PlayerSession` into this blueprint's own
/// `PlayerProfile`/`enter_play` call and spawns it as its own Tokio task, since
/// `PlayerSessionSink::accept` is synchronous while `enter_play` is `async` and runs for
/// the connection's remaining lifetime.
impl PlayerSessionSink for HardcodedWorld {
    fn accept(&self, session: PlayerSession) {
        let world = self.clone();
        let profile = PlayerProfile {
            uuid: session.profile.id.as_u128(),
            username: session.profile.name,
        };
        tokio::spawn(async move {
            enter_play(session.connection, session.inbound, profile, &world).await;
        });
    }
}

/// A fresh, disposable directory under `std::env::temp_dir()`, unique per call (process
/// id + a nanosecond timestamp + the calling thread's own id) -- `HardcodedWorld::new`'s
/// own backward-compatible zero-argument constructor's private world storage (that
/// method's own doc comment has the full reasoning).
fn unique_temp_world_dir() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rc-hardcoded-world-{}-{nanos}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ))
}
