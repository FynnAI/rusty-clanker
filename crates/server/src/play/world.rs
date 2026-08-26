//! The one hardcoded region and its 20 TPS tick loop -- this blueprint's own composition-
//! root wiring (M1-B05 blueprint Context, "The hardcoded region and its 20 TPS tick loop").
//! No `rc_scheduler::RegionManager` -- a single region that never splits or merges has no
//! use for its merge/split lifecycle; `RcExecutor::spawn_region` is called directly.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

use bevy_ecs::prelude::*;
use rc_chunk_storage::io_pool::ChunkNbtResolvers;
use rc_chunk_storage::lifecycle::ChunkLifecycleManager;
use rc_chunk_storage::superflat::SuperflatFiller;
use rc_chunk_storage::{
    AnvilDiskBackend, BiomeColumn, BlockStateColumn, ChunkKeyTag, ChunkStorageBackend,
    CompressionScheme, FilesystemPlayerDataStore, PaletteThresholds,
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
    RejectReason, apply_block_action, debug_query_block, eye_position_from_feet, target_position,
    to_storage_biome_id, to_storage_id, within_reach,
};
use super::connection::SPAWN_POSITION;
use super::movement::{PendingMovementUpdate, feet_block_pos};
use super::packets::{
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, ChunkBatchStart, LevelChunkWithLight,
    SetChunkCacheCenter, pack_position,
};
use super::persistence::PlayerSessionStore;
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
    /// New (M2 field-report movement-application fix): this player's own persisted-record
    /// uuid -- lets the tick loop sync a live position/rotation update straight back into
    /// `PlayerSessionStore` (`apply_movement_updates`'s own doc comment) without threading a
    /// second uuid-keyed lookup through every call site.
    pub uuid: uuid::Uuid,
    /// New (M2 field-report movement-application fix): this player's own authoritative
    /// position/rotation/on_ground state -- the exact "anywhere" a decoded `SetPlayerPosition`/
    /// `SetPlayerPositionAndRotation`/`SetPlayerRotation` never reached before this fix
    /// (`play::movement`'s own module doc comment has the full root-cause writeup). Always a
    /// real fractional value, never block-center-snapped (`eye_position_from_feet`'s own doc
    /// comment).
    pub position: [f64; 3],
    pub rotation: [f32; 2],
    pub on_ground: bool,
    /// New (M2 field-report chunk-streaming fix): the chunk key this player's own
    /// `SetChunkCacheCenter`/streaming ticket was last computed from --
    /// `stream_chunks_for_moved_players`'s own doc comment has the full cadence rule (only
    /// recomputed on an actual chunk-key change, matching vanilla's own
    /// `ClientboundSetChunkCacheCenterPacket` cadence -- `docs/research/mc-26.2/03-world-
    /// chunks.md`'s own restatement of `ChunkMap.applyChunkTrackingView`: "a
    /// `ClientboundSetChunkCacheCenterPacket` whenever the view's center chunk itself
    /// moved").
    pub last_streamed_center: ChunkKey,
    /// New (M2 field-report chunk-streaming fix): every chunk coordinate already sent to
    /// this connection as `LevelChunkWithLight` -- lets the streaming step compute exactly
    /// the "newly entered" set on each chunk crossing without resending anything already on
    /// the client's own chunk cache.
    pub sent_chunks: HashSet<(i32, i32)>,
}

pub struct PendingJoin {
    pub network_entity_id: i32,
    pub username: String,
    /// New -- carried from `enter_play`'s Tokio task across the same channel boundary
    /// `network_entity_id`/`username` already cross.
    pub connection: ConnectionHandle,
    /// New (M2 field-report movement-application/persistence fix): this player's own
    /// persisted-record uuid and the real, just-loaded (or freshly defaulted)
    /// position/rotation `enter_play` already sent as this same connection's own
    /// `SynchronizePlayerPosition` -- previously lost the moment `PendingJoin` crossed this
    /// channel boundary, forcing the tick-loop's own ticket registration (below) to fall
    /// back on the hardcoded `SPAWN_POSITION` for every join, including a rejoin far from
    /// spawn.
    pub uuid: uuid::Uuid,
    pub position: [f64; 3],
    pub rotation: [f32; 2],
}

/// M2 integration addition: one connection's request for a batch of real, storage-backed
/// chunk columns' wire-encoded content -- the bridge `connection.rs`'s `enter_play` needs
/// between "register a real ticket so these chunks start loading" and "hand back their
/// real, currently-resident content once loading (superflat-filled or disk-restored,
/// M2-B05's own async load path) has actually finished," replacing `chunk::
/// build_placeholder_chunk_data()`'s static blob (`M2-COMPLETION-REPORT.md`'s own
/// diagnosed gap). Private -- `HardcodedWorld::request_chunk_grid` is the only public
/// surface a caller ever needs.
struct ChunkGridRequest {
    network_entity_id: i32,
    center: ChunkKey,
    ticket_radius: u8,
    /// Absolute `(chunk_x, chunk_z)` pairs to encode and return, in the exact order the
    /// caller wants them sent back -- kept independent of `ticket_radius`/`center` so
    /// this type never assumes the requested grid and the ticket's own load radius share
    /// one shape.
    coords: Vec<(i32, i32)>,
    reply: oneshot::Sender<Vec<Vec<u8>>>,
}

/// The tick-loop-owned bookkeeping for one still-resolving `ChunkGridRequest` -- carried
/// across tick iterations exactly like `carried_block_actions` below, since the requested
/// chunks' async load may take several ticks to complete.
struct PendingChunkGridRequest {
    coords: Vec<(i32, i32)>,
    resolved: Vec<Option<Vec<u8>>>,
    reply: oneshot::Sender<Vec<Vec<u8>>>,
}

/// New (M2 field-report chunk-streaming fix): one chunk coordinate `stream_chunks_for_
/// moved_players` has already requested (ticket registered, `PlayerMarker::sent_chunks`
/// marked) for a specific player on a chunk crossing, still waiting for that column to
/// become resident -- the streaming-on-move analog of `PendingChunkGridRequest` above,
/// minus the oneshot-reply/whole-request-batch machinery that only Play-entry's own
/// bulk-wait-then-reply shape needs. Carried across tick iterations exactly like
/// `carried_block_actions`/`chunk_grid_requests` -- the requested chunk's async load may
/// take several ticks to complete.
struct PendingStreamChunk {
    network_entity_id: i32,
    coord: (i32, i32),
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
    /// New (M2 field-report movement-application fix): enqueued by `connection.rs`'s
    /// inbound dispatch on every decoded `SetPlayerPosition`/`SetPlayerPositionAndRotation`/
    /// `SetPlayerRotation`, drained once per tick alongside `block_action_tx` above
    /// (`apply_movement_updates`'s own doc comment).
    movement_tx: tokio::sync::mpsc::UnboundedSender<PendingMovementUpdate>,
    /// New (M2-B07), test/diagnostic only -- `debug_query_block`'s own doc comment.
    query_tx:
        tokio::sync::mpsc::UnboundedSender<(BlockPos, oneshot::Sender<Option<DebugBlockInfo>>)>,
    next_network_entity_id: Arc<AtomicI32>,
    /// New (M2-B05): signals the region thread to stop after finishing its current round
    /// (`shutdown`'s own doc comment).
    shutdown_flag: Arc<AtomicBool>,
    /// New (M2-B05): the region thread's own `JoinHandle`, taken and joined by `shutdown`.
    thread_handle: Arc<Mutex<Option<std::thread::JoinHandle<()>>>>,
    /// New (M2 integration): `connection.rs`'s own entry point into `ChunkGridRequest`
    /// (`request_chunk_grid`'s own doc comment).
    chunk_grid_tx: tokio::sync::mpsc::UnboundedSender<ChunkGridRequest>,
    /// New (M2 integration, M2-B06's own "Composition-root integration" recipe step 1):
    /// this world's player-record working set -- `player_sessions()`'s own doc comment.
    sessions: PlayerSessionStore,
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
        let (movement_tx, mut movement_rx) =
            tokio::sync::mpsc::unbounded_channel::<PendingMovementUpdate>();
        let (query_tx, mut query_rx) = tokio::sync::mpsc::unbounded_channel::<(
            BlockPos,
            oneshot::Sender<Option<DebugBlockInfo>>,
        )>();
        let (chunk_grid_tx, mut chunk_grid_rx) =
            tokio::sync::mpsc::unbounded_channel::<ChunkGridRequest>();
        let shutdown_flag = Arc::new(AtomicBool::new(false));

        // M2 integration addition (M2-B06's own "Composition-root integration" recipe
        // step 1): built here, before `config` moves into the tick-loop closure below, so
        // both the returned handle and the tick thread itself can each hold their own
        // clone (`PlayerSessionStore` is `Clone`, cheap, `Arc`-backed).
        let sessions = PlayerSessionStore::new(Arc::new(FilesystemPlayerDataStore::new(
            config.world_dir.clone(),
        )));
        let sessions_for_thread = sessions.clone();

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
            // M2 integration addition: still-resolving `ChunkGridRequest`s, carried across
            // tick iterations exactly like `carried_block_actions` above (`request_chunk_
            // grid`'s own doc comment -- the requested chunks' async load may take several
            // ticks to complete).
            let mut chunk_grid_requests: Vec<PendingChunkGridRequest> = Vec::new();
            // New (M2 field-report movement-application fix): a movement update whose own
            // `network_entity_id` has no `PlayerMarker` spawned in `region.world` yet this
            // tick (the same join/action mpsc-ordering race `task_9ce21947` flagged for
            // block actions, `respond_to_action`'s own doc comment) is carried into the
            // next tick's own drain instead of being silently dropped.
            let mut carried_movement_updates: Vec<PendingMovementUpdate> = Vec::new();
            // New (M2 field-report chunk-streaming fix): every chunk coordinate `stream_
            // chunks_for_moved_players` has requested for a moved player but that has not
            // yet become resident, carried across tick iterations exactly like `carried_
            // block_actions`/`chunk_grid_requests` above.
            let mut pending_stream_chunks: Vec<PendingStreamChunk> = Vec::new();
            // M2 integration addition (M2-B06's own "Composition-root integration" recipe
            // step 4): a plain per-tick counter driving the periodic player-data save
            // sweep, reusing the same configured interval `lifecycle`'s own Stage-9 chunk
            // cadence already uses (`config.save_interval_ticks()`) rather than a second,
            // independently-configured one.
            let player_save_interval_ticks = config.save_interval_ticks() as u64;
            let mut player_save_tick_counter: u64 = 0;

            loop {
                if thread_shutdown_flag.load(Ordering::Relaxed) {
                    lifecycle.shutdown(&region.world);
                    // M2 integration addition (M2-B06's own "Composition-root
                    // integration" recipe step 5): gives every still-connected player's
                    // data the same clean-restart guarantee WORLD-D25 already gives
                    // chunk data.
                    sessions_for_thread.save_all();
                    return;
                }

                while let Ok(join) = join_rx.try_recv() {
                    // M2 field-report fix: this player's own real, just-loaded (or freshly
                    // defaulted) chunk -- previously this registration unconditionally used
                    // `SPAWN_POSITION`'s own chunk plus `config.simulation_distance_chunks`
                    // (a leftover of this join-drain step's own pre-M2-B06 shape), clobbering
                    // the correct, already-registered ticket `connection.rs`'s own
                    // `request_chunk_grid` call (`chunk_grid_rx`'s own drain, below) had
                    // already set up moments earlier for this exact player at this exact
                    // radius -- harmless only by coincidence for a first-ever join at
                    // `SPAWN_POSITION` itself, but silently reset a rejoining player's own
                    // ticket back to spawn on every reconnect, undoing this same fix's own
                    // persistence consumer the moment a player actually rejoined away from
                    // spawn. Registers (replaces, harmlessly -- `register_player`'s own doc
                    // comment) at the identical center/radius `request_chunk_grid` already
                    // used, so this call is now a no-op in the common case and a correct,
                    // defensive re-assertion in every case (e.g. a future entry point that
                    // skips `request_chunk_grid`).
                    let join_chunk =
                        feet_block_pos(join.position).chunk_key(DimensionId::OVERWORLD);
                    ticket_manager.register_player(
                        PlayerTicketId(join.network_entity_id),
                        join_chunk,
                        chunk::PLACEHOLDER_RADIUS_CHUNKS as u8,
                    );
                    // Every chunk coordinate `connection.rs`'s own Play-entry sequence
                    // already sent for this player (`enter_play`'s own `coords`, computed
                    // identically from the same `join.position`) -- so `stream_chunks_for_
                    // moved_players` (below) only ever streams genuinely new coordinates.
                    let already_sent: HashSet<(i32, i32)> = chunk::placeholder_chunk_coords()
                        .into_iter()
                        .map(|(dx, dz)| (join_chunk.x + dx, join_chunk.z + dz))
                        .collect();
                    region.world.spawn(PlayerMarker {
                        network_entity_id: join.network_entity_id,
                        username: join.username,
                        connection: join.connection,
                        uuid: join.uuid,
                        position: join.position,
                        rotation: join.rotation,
                        on_ground: true,
                        last_streamed_center: join_chunk,
                        sent_chunks: already_sent,
                    });
                }

                // M2 integration addition: registers (or replaces, harmlessly -- Context's
                // own `register_player` doc comment) a real ticket for every newly arrived
                // `ChunkGridRequest` immediately, so this same tick's `ticket_manager.step()`
                // call below already starts loading the requested grid -- no need to wait
                // for the (separate, later) `PendingJoin` drain above.
                while let Ok(req) = chunk_grid_rx.try_recv() {
                    ticket_manager.register_player(
                        PlayerTicketId(req.network_entity_id),
                        req.center,
                        req.ticket_radius,
                    );
                    chunk_grid_requests.push(PendingChunkGridRequest {
                        resolved: vec![None; req.coords.len()],
                        coords: req.coords,
                        reply: req.reply,
                    });
                }

                // M2 field-report fix: drain and apply every pending movement update
                // (`play::movement`'s own module doc comment has the full root-cause
                // writeup) -- previously nothing in this tick loop ever touched
                // `movement_rx` at all, so a decoded `SetPlayerPosition`/
                // `SetPlayerPositionAndRotation`/`SetPlayerRotation` (`connection.rs`'s own
                // dispatch arms) had nowhere to go. Applied before `ticket_manager.step()`
                // (below) so this same tick's own churn computation already reflects any
                // chunk-crossing move (`stream_chunks_for_moved_players`'s own doc
                // comment, further down).
                let mut movement_updates: Vec<PendingMovementUpdate> =
                    std::mem::take(&mut carried_movement_updates);
                while let Ok(update) = movement_rx.try_recv() {
                    movement_updates.push(update);
                }
                for update in movement_updates {
                    let mut query = region.world.query::<&mut PlayerMarker>();
                    let Some(mut marker) = query
                        .iter_mut(&mut region.world)
                        .find(|marker| marker.network_entity_id == update.network_entity_id)
                    else {
                        // The same join/action mpsc-ordering race `task_9ce21947` flagged
                        // for block actions (`respond_to_action`'s own doc comment) -- this
                        // player's own `PlayerMarker` has not been spawned into
                        // `region.world` yet this tick. Carried into the next tick's own
                        // drain rather than dropped.
                        carried_movement_updates.push(update);
                        continue;
                    };
                    if let Some(position) = update.position {
                        marker.position = position;
                    }
                    if let Some(rotation) = update.rotation {
                        marker.rotation = rotation;
                    }
                    if let Some(on_ground) = update.on_ground {
                        marker.on_ground = on_ground;
                    }
                    if update.position.is_some() || update.rotation.is_some() {
                        // M2 field-report persistence fix: syncs the live position/rotation
                        // straight back into this player's own session record on every
                        // applied update, not only at disconnect -- `sessions_for_thread.
                        // save_all()`'s own periodic sweep (below) and `SaveOnDisconnect`'s
                        // own disconnect-time save (`connection.rs`) both simply persist
                        // whatever this record currently holds, so both are only ever as
                        // fresh as this sync. AC1c's own "player rejoins at the position
                        // they left at" fix.
                        let uuid = marker.uuid;
                        let position = marker.position;
                        let rotation = marker.rotation;
                        sessions_for_thread.with_record_mut(uuid, |record| {
                            record.data.pos = position;
                            record.data.rotation = rotation;
                        });
                    }
                }

                // M2 field-report chunk-streaming fix: recomputes every currently-spawned
                // player's own current chunk and, for any whose chunk changed since the
                // last time this ran, re-centers their ticket (`TicketManager::move_player`
                // -- that method's own doc comment: "no production call site exists at M2...
                // exposed for a future mechanics blueprint," this is that call site now),
                // sends the matching `SetChunkCacheCenter` update (cadence: only on an
                // actual chunk-key change, `PlayerMarker::last_streamed_center`'s own doc
                // comment -- matches vanilla's own `ClientboundSetChunkCacheCenterPacket`
                // cadence), and queues every newly-visible, not-yet-sent chunk coordinate
                // for this tick's own streaming resolution step (below, after the
                // `ChunkIndex` refresh). Runs every tick unconditionally -- cheap
                // (`O(players)`) and a correct no-op whenever nobody's chunk changed. This
                // is AC1b's own "walking N chunks streams new chunks in" fix.
                let mut moved_query = region.world.query::<&mut PlayerMarker>();
                for mut marker in moved_query.iter_mut(&mut region.world) {
                    let current_chunk =
                        feet_block_pos(marker.position).chunk_key(DimensionId::OVERWORLD);
                    if current_chunk == marker.last_streamed_center {
                        continue;
                    }
                    marker.last_streamed_center = current_chunk;
                    ticket_manager
                        .move_player(PlayerTicketId(marker.network_entity_id), current_chunk);
                    let _ =
                        marker
                            .connection
                            .try_send_payload(encode_payload(&SetChunkCacheCenter {
                                chunk_x: current_chunk.x,
                                chunk_z: current_chunk.z,
                            }));
                    for (dx, dz) in chunk::placeholder_chunk_coords() {
                        let coord = (current_chunk.x + dx, current_chunk.z + dz);
                        if marker.sent_chunks.insert(coord) {
                            pending_stream_chunks.push(PendingStreamChunk {
                                network_entity_id: marker.network_entity_id,
                                coord,
                            });
                        }
                    }
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

                // M2 integration addition: resolves every still-pending `ChunkGridRequest`
                // against this tick's own fresh `ChunkIndex` residency directory (just
                // inserted above) -- encodes (`chunk::encode_live_chunk_data`) and records
                // each requested coordinate's real content the first tick its chunk
                // entity becomes resident, and replies once every coordinate in a request
                // has resolved. A request whose grid has not fully loaded yet simply stays
                // in `chunk_grid_requests` for a later tick (mirrors `carried_block_
                // actions`' own carry-forward pattern above). Looks each needed key up
                // directly off the resource (no whole-map clone) -- this block is a no-op,
                // zero-cost past the empty check, once every request has resolved, so a
                // long-running region never pays a per-tick tax for chunk-grid streaming
                // it isn't currently doing.
                if !chunk_grid_requests.is_empty() {
                    let mut i = 0;
                    while i < chunk_grid_requests.len() {
                        {
                            let req = &mut chunk_grid_requests[i];
                            for slot in 0..req.coords.len() {
                                if req.resolved[slot].is_some() {
                                    continue;
                                }
                                let (cx, cz) = req.coords[slot];
                                let key = ChunkKey::new(DimensionId::OVERWORLD, cx, cz);
                                let entity =
                                    region.world.resource::<ChunkIndex>().0.get(&key).copied();
                                if let Some(entity) = entity {
                                    let blocks = region.world.get::<BlockStateColumn>(entity).expect(
                                        "every resident chunk entity carries BlockStateColumn (M2-B01's fixed component set)",
                                    );
                                    let biomes = region.world.get::<BiomeColumn>(entity).expect(
                                        "every resident chunk entity carries BiomeColumn (M2-B01's fixed component set)",
                                    );
                                    req.resolved[slot] =
                                        Some(chunk::encode_live_chunk_data(blocks, biomes));
                                }
                            }
                        }
                        if chunk_grid_requests[i].resolved.iter().all(Option::is_some) {
                            let req = chunk_grid_requests.remove(i);
                            let ordered: Vec<Vec<u8>> = req
                                .resolved
                                .into_iter()
                                .map(|v| v.expect("just checked all Some"))
                                .collect();
                            let _ = req.reply.send(ordered);
                        } else {
                            i += 1;
                        }
                    }
                }

                // M2 field-report chunk-streaming fix: resolves every still-pending
                // streamed chunk (`stream_chunks_for_moved_players`'s own doc comment,
                // above) against this tick's own fresh `ChunkIndex` residency directory --
                // encodes and sends each one the first tick its chunk entity becomes
                // resident, batched per player behind `ChunkBatchStart`/`ChunkBatchFinished`
                // (mirroring `enter_play`'s own Play-entry framing exactly -- ordinary
                // post-login streaming uses the identical batch framing on a real vanilla
                // server, `docs/research/mc-26.2/03-world-chunks.md`'s own restatement of
                // `ChunkMap.applyChunkTrackingView`). A not-yet-resident coordinate simply
                // stays in `pending_stream_chunks` for a later tick (mirrors `chunk_grid_
                // requests`' own carry-forward pattern above).
                if !pending_stream_chunks.is_empty() {
                    let mut ready: std::collections::HashMap<i32, Vec<(i32, i32, Vec<u8>)>> =
                        std::collections::HashMap::new();
                    let mut still_pending = Vec::new();
                    for item in std::mem::take(&mut pending_stream_chunks) {
                        let key = ChunkKey::new(DimensionId::OVERWORLD, item.coord.0, item.coord.1);
                        let entity = region.world.resource::<ChunkIndex>().0.get(&key).copied();
                        match entity {
                            Some(entity) => {
                                let blocks = region.world.get::<BlockStateColumn>(entity).expect(
                                    "every resident chunk entity carries BlockStateColumn (M2-B01's fixed component set)",
                                );
                                let biomes = region.world.get::<BiomeColumn>(entity).expect(
                                    "every resident chunk entity carries BiomeColumn (M2-B01's fixed component set)",
                                );
                                let data = chunk::encode_live_chunk_data(blocks, biomes);
                                ready.entry(item.network_entity_id).or_default().push((
                                    item.coord.0,
                                    item.coord.1,
                                    data,
                                ));
                            }
                            None => still_pending.push(item),
                        }
                    }
                    pending_stream_chunks = still_pending;

                    if !ready.is_empty() {
                        let heightmaps = chunk::build_placeholder_heightmaps();
                        let (
                            sky_light_mask,
                            block_light_mask,
                            empty_sky_light_mask,
                            empty_block_light_mask,
                            sky_light_arrays,
                            block_light_arrays,
                        ) = chunk::build_placeholder_light();
                        for entity_ref in region.world.iter_entities() {
                            let Some(marker) = entity_ref.get::<PlayerMarker>() else {
                                continue;
                            };
                            let Some(chunks) = ready.remove(&marker.network_entity_id) else {
                                continue;
                            };
                            let batch_size = chunks.len() as i32;
                            let _ = marker
                                .connection
                                .try_send_payload(encode_payload(&ChunkBatchStart {}));
                            for (chunk_x, chunk_z, data) in chunks {
                                let level_chunk = LevelChunkWithLight {
                                    chunk_x,
                                    chunk_z,
                                    heightmaps: heightmaps.clone(),
                                    data,
                                    block_entities: Vec::new(),
                                    sky_light_mask: sky_light_mask.clone(),
                                    block_light_mask: block_light_mask.clone(),
                                    empty_sky_light_mask: empty_sky_light_mask.clone(),
                                    empty_block_light_mask: empty_block_light_mask.clone(),
                                    sky_light_arrays: sky_light_arrays.clone(),
                                    block_light_arrays: block_light_arrays.clone(),
                                };
                                let _ = marker
                                    .connection
                                    .try_send_payload(encode_payload(&level_chunk));
                            }
                            let _ = marker.connection.try_send_payload(encode_payload(
                                &ChunkBatchFinished { batch_size },
                            ));
                        }
                        // Any player whose connection has since dropped (`ready` still
                        // holding entries after every current `PlayerMarker` was checked)
                        // simply has its already-resident chunk data discarded here --
                        // matches `chunk_grid_requests`' own `let _ = req.reply.send(..)`
                        // best-effort semantics for a reply nobody is left to receive.
                    }
                }

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
                    // M2 field-report fix: reach validation now keys off the acting
                    // player's own live position (`PlayerMarker::position`, kept current by
                    // the movement-application step above) instead of the hardcoded
                    // `SPAWN_POSITION` constant every previous version of this check used
                    // unconditionally -- the fix for the reported "place/break only works
                    // in a sphere around spawn" symptom (everything beyond `BLOCK_
                    // INTERACTION_RANGE_CREATIVE` of `SPAWN_POSITION` was rejected as
                    // `OutOfReach` no matter where the player actually stood). Falls back to
                    // `SPAWN_POSITION` for the same join/action mpsc-ordering race `respond_
                    // to_action`'s own fallback handles (the actor's own `PlayerMarker` has
                    // not been spawned into `region.world` yet this tick) -- never panics on
                    // a not-yet-spawned actor.
                    let actor_position =
                        player_feet_position(&region.world, action.network_entity_id).unwrap_or([
                            SPAWN_POSITION.x as f64,
                            SPAWN_POSITION.y as f64,
                            SPAWN_POSITION.z as f64,
                        ]);
                    let out_of_reach = target.is_some_and(|target| {
                        !within_reach(
                            eye_position_from_feet(actor_position),
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

                // M2 integration addition (M2-B06's own "Composition-root integration"
                // recipe step 4): the periodic player-data save sweep -- a plain,
                // uncoordinated background thread (Context's own "Documented, bounded
                // simplification": `RC-IoPool` does not exist as a player-data-save target
                // at M2's own scope), never blocking this tick loop itself.
                player_save_tick_counter += 1;
                if player_save_tick_counter >= player_save_interval_ticks {
                    player_save_tick_counter = 0;
                    let sessions_for_sweep = sessions_for_thread.clone();
                    std::thread::spawn(move || sessions_for_sweep.save_all());
                }

                clock.await_next_tick();
            }
        });

        Self {
            join_tx,
            block_action_tx,
            movement_tx,
            query_tx,
            next_network_entity_id: Arc::new(AtomicI32::new(1)),
            shutdown_flag,
            thread_handle: Arc::new(Mutex::new(Some(handle))),
            chunk_grid_tx,
            sessions,
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

    /// New (M2 field-report movement-application fix). Enqueues a decoded movement claim,
    /// applied at the start of this region's next tick (`apply_movement_updates`'s own doc
    /// comment). Never blocks.
    pub fn queue_movement(&self, update: PendingMovementUpdate) {
        self.movement_tx
            .send(update)
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

    /// New (M2 integration): registers a real ticket for `network_entity_id` centered on
    /// `center` with `ticket_radius`, then waits -- across as many ticks as the async
    /// load pipeline needs (M2-B05's own load path) -- for every one of `coords`' chunk
    /// columns to actually become resident, and returns their real, currently-live
    /// block/biome content already wire-encoded (`chunk::encode_live_chunk_data`), in the
    /// same order as `coords`. Replaces `chunk::build_placeholder_chunk_data()`'s static,
    /// always-identical blob (`M2-COMPLETION-REPORT.md`'s own diagnosed gap). Never
    /// blocks the caller's own OS thread -- awaits a oneshot reply the tick thread
    /// fulfils once every requested chunk is resident.
    pub async fn request_chunk_grid(
        &self,
        network_entity_id: i32,
        center: ChunkKey,
        ticket_radius: u8,
        coords: Vec<(i32, i32)>,
    ) -> Vec<Vec<u8>> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.chunk_grid_tx
            .send(ChunkGridRequest {
                network_entity_id,
                center,
                ticket_radius,
                coords,
                reply: reply_tx,
            })
            .expect("the hardcoded region's tick-loop thread outlives every connection");
        reply_rx.await.expect(
            "the hardcoded region's tick-loop thread always replies before dropping the sender",
        )
    }

    /// New (M2 integration, M2-B06's own "Composition-root integration" recipe step 2/3):
    /// exposes this world's player-record working set to `enter_play`. `Clone`, cheap
    /// (`Arc`-backed), matching every other `HardcodedWorld` handle method's own shape.
    pub fn player_sessions(&self) -> PlayerSessionStore {
        self.sessions.clone()
    }
}

/// New (M2 field-report movement-application fix): the live position of the `PlayerMarker`
/// whose `network_entity_id` matches `network_entity_id`, if currently spawned in `world` --
/// the reach-check consumer's own lookup (the tick loop's own block-action processing
/// step, above). `None` for the same join/action mpsc-ordering race `respond_to_action`'s
/// own fallback handles (never panics on a not-yet-spawned actor).
fn player_feet_position(world: &World, network_entity_id: i32) -> Option<[f64; 3]> {
    world.iter_entities().find_map(|entity_ref| {
        let marker = entity_ref.get::<PlayerMarker>()?;
        (marker.network_entity_id == network_entity_id).then_some(marker.position)
    })
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
            // M2 field-report fix (task_9ce21947): the acting player's own `PlayerMarker`
            // may not be spawned into `world` yet this same tick -- two independent mpsc
            // channels (`HardcodedWorld::join_tx`/`block_action_tx`) race, with no
            // guarantee that a join enqueued moments before this same action's own packet
            // has already been drained into `region.world` by the time this action is
            // processed. Broadcasting purely by iterating `world`'s own `PlayerMarker`s
            // silently dropped exactly this actor's own copy in that case (every *other*
            // already-spawned player still received theirs correctly, since only the
            // actor's entity could possibly be missing this tick). `actor_reached` tracks
            // whether the iteration below already found and sent to the actor's own
            // connection; if not, it is sent once more directly via `action.connection`
            // (already carried on `PendingBlockAction`, exactly like the unconditional ack
            // above) -- guaranteeing the actor is reached regardless of spawn ordering,
            // without ever double-sending when their `PlayerMarker` already existed.
            let mut actor_reached = false;
            for entity_ref in world.iter_entities() {
                if let Some(marker) = entity_ref.get::<PlayerMarker>() {
                    let _ = marker.connection.try_send_payload(payload.clone());
                    if marker.network_entity_id == action.network_entity_id {
                        actor_reached = true;
                    }
                }
            }
            if !actor_reached {
                let _ = action.connection.try_send_payload(payload);
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
