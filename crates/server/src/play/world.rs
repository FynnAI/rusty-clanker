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
    AnvilDiskBackend, BiomeColumn, BlockStateColumn, ChunkKeyTag, ChunkPersistenceState,
    ChunkStorageBackend, CompressionScheme, FilesystemPlayerDataStore, PaletteThresholds,
    WORLD_HEIGHT, WORLD_MIN_Y,
};
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_messaging::{Address, RegionId, RegionMessage, RegionMessageBus};
use rc_physics::{Aabb, PLAYER_HALF_WIDTH, PLAYER_HEIGHT, PLAYER_HEIGHT_SNEAKING, Vec3};
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
    BlockActionKind, ChunkIndex, DebugBlockInfo, PendingBlockAction, debug_query_block,
    target_position, to_storage_biome_id, to_storage_id,
};
use super::connection::SPAWN_POSITION;
use super::mining::{
    self, BLOCK_INTERACTION_RANGE_CREATIVE, BLOCK_INTERACTION_RANGE_SURVIVAL, BreakOutcome,
    DestroyOutcome, DestroyState, GameModeState, HeldItem, HeldItemStub, PlaceOutcome,
    PlaceableBlockKind, StopOutcome, TickOutcome, ToolKind, ToolMaterial,
};
use super::movement::{
    ChunkBlockShapeSource, MovementOutcome, PendingMoveReport, PendingMovementPacket,
    PendingPlayerInput, PlayerInputState, PlayerMotion, TeleportState, evaluate_movement,
    eye_position, feet_block_pos, merge_move_report,
};
use super::packets::{
    AcknowledgeBlockChange, BlockUpdate, ChunkBatchFinished, ChunkBatchStart,
    LEVEL_EVENT_BLOCK_BREAK, LevelChunkWithLight, LevelEvent, SetBlockDestroyStage,
    SetChunkCacheCenter, SynchronizePlayerPosition, pack_position,
};
use super::persistence::PlayerSessionStore;
use super::registry_resolvers::McRegistryResolvers;
use super::{PlayerProfile, chunk, enter_play};
use crate::config::WorldConfig;
use crate::net::{ConnectionHandle, PlayerSession, PlayerSessionSink};
use rc_chunk_storage::RegistryId as _;
use rc_mechanics::{BlockWorldAccess, RegionOwnership};

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
    // M3-B03 (Context, "Wiring M3-B01's Stage-4 substrate into `HardcodedWorld` for the
    // first time"): inserts the six `Default`-able Stage-4 resources -- `RegionOwnership`
    // (the seventh, per that same Context section) has no sensible uniform default and is
    // instead inserted once, per region, immediately after `RcExecutor::spawn_region`
    // returns (`with_config`'s own construction sequence, below).
    rc_mechanics::stage4::ecs::bootstrap_default_stage4_resources(world);
}

/// Direct (non-`Query`) `BlockWorldAccess` adapter over `region.world`'s own chunk entities
/// and `block_action::ChunkIndex` directory -- the tick loop's own Stage-3-equivalent manual
/// step runs outside any `bevy_ecs::System`, so it cannot use `stage4::ecs::EcsBlockWorld`'s
/// own `Query`-based construction (mirrors `movement.rs`'s own `ChunkBlockShapeSource`
/// precedent for the identical reason). `local`/`dimension` are enough to answer
/// `owner_of`/`local_identity` honestly at this milestone's own single-region scope (Context:
/// "M2 stays inside M1-B05's single `HARDCODED_REGION_ID`", still true) without needing a
/// real `RegionOwnership` instance threaded through here too.
struct DirectBlockWorld<'w> {
    world: &'w mut World,
    dimension: DimensionId,
    local: Address,
}

/// `true` iff `world_y` falls inside the pinned world's vertical bounds (`WORLD_MIN_Y ..
/// WORLD_MIN_Y + WORLD_HEIGHT`). `BlockPos` itself performs no such validation (Context,
/// `rc-core`'s own doc comment on that type) and `rc-chunk-storage`'s own `BlockStateColumn`
/// accessors `assert!` instead of returning gracefully (`column.rs`'s own documented
/// contract, load-bearing for that crate's other callers) -- every `BlockWorldAccess`
/// implementation that ultimately reaches those accessors (this one; `stage4::ecs::
/// EcsBlockWorld`, mirrored) must therefore pre-check here, at exactly this boundary, so a
/// position derived from a neighbour/offset that lands outside the world (breaking or
/// placing a block at the world floor/ceiling, MECH-D-adjacent fan-out) resolves as "not
/// present" instead of ever reaching the asserting accessor. Mirrors `movement.rs`'s own
/// established `pos.y < WORLD_MIN_Y || pos.y >= WORLD_MIN_Y + WORLD_HEIGHT` check.
fn y_in_world_bounds(world_y: i32) -> bool {
    (WORLD_MIN_Y..WORLD_MIN_Y + WORLD_HEIGHT).contains(&world_y)
}

impl BlockWorldAccess for DirectBlockWorld<'_> {
    fn get_block(&self, pos: BlockPos) -> Option<rc_chunk_storage::BlockStateId> {
        if !y_in_world_bounds(pos.y) {
            return None;
        }
        let key = pos.chunk_key(self.dimension);
        let entity = *self.world.resource::<ChunkIndex>().0.get(&key)?;
        let column = self.world.get::<BlockStateColumn>(entity)?;
        let (lx, lz) = (pos.x.rem_euclid(16) as u8, pos.z.rem_euclid(16) as u8);
        Some(column.get(lx, pos.y, lz))
    }

    fn set_block(&mut self, pos: BlockPos, state: rc_chunk_storage::BlockStateId) -> bool {
        if !y_in_world_bounds(pos.y) {
            // Vanilla parity (Context (c)'s field-report note): a write beyond the world's
            // own vertical bounds is simply dropped, never an error and never propagated --
            // no fan-out follows since `UpdateContext::set_block`'s caller only observes
            // `false`, indistinguishable from any other already-established no-op write.
            return false;
        }
        let key = pos.chunk_key(self.dimension);
        let Some(entity) = self.world.resource::<ChunkIndex>().0.get(&key).copied() else {
            return false;
        };
        let mut entity_mut = self.world.entity_mut(entity);
        let (lx, lz) = (pos.x.rem_euclid(16) as u8, pos.z.rem_euclid(16) as u8);
        let changed = entity_mut
            .get_mut::<BlockStateColumn>()
            .map(|mut column| column.set(lx, pos.y, lz, state))
            .unwrap_or(false);
        if changed {
            // Preserves M2-B07's own persistence guarantee (`Rejected`-free writes always
            // dirty-mark their chunk) -- the real-client-verified "changes persist across
            // restart" behavior (M2's own real-client test) must keep working through this
            // blueprint's own new mutation path too.
            if let Some(mut persistence) = entity_mut.get_mut::<ChunkPersistenceState>() {
                persistence.mark_dirty();
            }
        }
        changed
    }

    fn dimension(&self) -> DimensionId {
        self.dimension
    }

    fn owner_of(&self, _chunk: ChunkKey) -> Address {
        self.local
    }

    fn local_identity(&self) -> Address {
        self.local
    }
}

/// Scans `world`'s currently-spawned `PlayerMarker`s for `network_entity_id` -- mirrors
/// `player_feet_position`'s own identical scan (below), generalized to return the whole
/// `Entity` so a caller can also reach that same player's `PlayerMotion`/`GameModeState`/
/// `HeldItem`/`DestroyState` components.
fn find_player_entity(world: &World, network_entity_id: i32) -> Option<Entity> {
    world.iter_entities().find_map(|entity_ref| {
        let marker = entity_ref.get::<PlayerMarker>()?;
        (marker.network_entity_id == network_entity_id).then_some(entity_ref.id())
    })
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
    /// M3-B02 (superseding the M2 field-report movement-application fix's own
    /// `PendingMovementUpdate`-typed channel): enqueued by `connection.rs`'s inbound
    /// dispatch on every decoded movement packet -- the four serverbound movement packets
    /// plus `ConfirmTeleportation` -- drained once per tick alongside `block_action_tx`
    /// above, after this same tick's own block-action drain-and-apply step (Context,
    /// "Which pipeline stage").
    movement_tx: tokio::sync::mpsc::UnboundedSender<PendingMovementPacket>,
    /// M3 field-report fix (Symptom 2): enqueued by `connection.rs`'s inbound dispatch on
    /// every decoded `player_input` packet, drained once per tick -- ahead of the block-action
    /// drain-and-apply step (unlike `movement_tx`), so this tick's own reach checks already
    /// see the latest sneak state (`queue_player_input`'s own doc comment).
    player_input_tx: tokio::sync::mpsc::UnboundedSender<PendingPlayerInput>,
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
    /// New (M3-B03), test/diagnostic only -- `debug_set_held_item`'s own doc comment.
    debug_held_item_tx:
        tokio::sync::mpsc::UnboundedSender<(i32, HeldItemStub, oneshot::Sender<()>)>,
    /// New (M3-B03), test/diagnostic only -- `debug_set_survival`'s own doc comment.
    debug_survival_tx: tokio::sync::mpsc::UnboundedSender<(i32, bool, oneshot::Sender<()>)>,
    /// New (M3-B03), test/diagnostic only -- `debug_stage4_counters`'s own doc comment.
    stage4_counters_tx: tokio::sync::mpsc::UnboundedSender<oneshot::Sender<Stage4Counters>>,
}

/// M3 field-report fix (symptom 2): `HardcodedWorld`'s per-connection channel methods
/// (`queue_join`/`queue_block_action`/`queue_movement_packet`/`queue_player_input`/
/// `request_chunk_grid`) return this instead of panicking once the hardcoded region's own
/// tick-loop thread has died -- this project's single hardcoded region has no supervision
/// or restart (out of scope, Context (c)), so every sender/oneshot reply that thread used
/// to own is gone for the rest of the process's life. Vanilla has no equivalent: a real
/// dedicated server simply exits on an unrecoverable tick-loop panic. This project instead
/// keeps already-open ports responsive, so a dead region must degrade every further
/// connection attempt gracefully (close/refuse with a diagnostic) rather than panic the
/// per-connection tokio task that happens to touch it next -- `connection.rs`'s own
/// established `if send.is_err() { return; }` idiom, extended to this failure mode too.
#[derive(Debug)]
pub struct RegionUnavailable;

impl std::fmt::Display for RegionUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the hardcoded region's tick-loop thread is no longer running"
        )
    }
}

impl std::error::Error for RegionUnavailable {}

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
            tokio::sync::mpsc::unbounded_channel::<PendingMovementPacket>();
        let (player_input_tx, mut player_input_rx) =
            tokio::sync::mpsc::unbounded_channel::<PendingPlayerInput>();
        let (query_tx, mut query_rx) = tokio::sync::mpsc::unbounded_channel::<(
            BlockPos,
            oneshot::Sender<Option<DebugBlockInfo>>,
        )>();
        let (chunk_grid_tx, mut chunk_grid_rx) =
            tokio::sync::mpsc::unbounded_channel::<ChunkGridRequest>();
        let (debug_held_item_tx, mut debug_held_item_rx) =
            tokio::sync::mpsc::unbounded_channel::<(i32, HeldItemStub, oneshot::Sender<()>)>();
        let (debug_survival_tx, mut debug_survival_rx) =
            tokio::sync::mpsc::unbounded_channel::<(i32, bool, oneshot::Sender<()>)>();
        let (stage4_counters_tx, mut stage4_counters_rx) =
            tokio::sync::mpsc::unbounded_channel::<oneshot::Sender<Stage4Counters>>();
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
            // M3-B03 (Context, "Wiring M3-B01's Stage-4 substrate"): registers M3-B01's two
            // Stage-4 systems into `DomainGroup::BlockRedstone` -- Stage 4 now runs for real
            // every tick (inert in the steady state under this milestone's own tier-1 scope,
            // `mining_stage4_wiring.rs`'s own acceptance test).
            rc_mechanics::stage4::ecs::register_stage4(&mut builder);
            let executor = builder.build().expect(
                "the Stage-9 snapshot system never violates ARCH-D8's structural-write check",
            );
            let mut region = executor.spawn_region(HARDCODED_REGION_ID);
            lifecycle.install_resources(&mut region.world);
            // M3-B03 (Context, same section): `RegionOwnership` has no sensible uniform
            // default (its own `resolve` closure is inherently per-region data) so it is
            // inserted here, once, immediately after `spawn_region` returns -- mirrors
            // M2-B07's own already-established `resolve_owner` closure shape verbatim, now
            // as M3-B01's own `RegionOwnership` type. Every chunk key is trivially local
            // (Context: "M2 stays inside M1-B05's single `HARDCODED_REGION_ID`", still true
            // at M3's own scope -- ARCH-D24's own real directory remains a later blueprint's
            // job).
            region.world.insert_resource(RegionOwnership {
                local: Address::Region(HARDCODED_REGION_ID),
                resolve: Box::new(|_key: ChunkKey| Address::Region(HARDCODED_REGION_ID)),
            });

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
            // M3-B02 (mirrors the M2 field-report movement-application fix's own identical
            // pattern): a movement report whose own `network_entity_id` has no
            // `PlayerMarker` spawned in `region.world` yet this tick (the same join/action
            // mpsc-ordering race `task_9ce21947` flagged for block actions, `respond_to_
            // action`'s own doc comment) is carried into the next tick's own drain instead
            // of being silently dropped.
            let mut carried_movement_updates: Vec<PendingMovementPacket> = Vec::new();
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
                    // M3-B02: spawns `PlayerMotion`/`TeleportState` alongside `PlayerMarker`
                    // in the same bundle -- `PlayerMotion`'s own initial position/rotation
                    // uses this player's real just-loaded (or freshly defaulted) `join.
                    // position`/`join.rotation`, not blindly the blueprint's own literal
                    // `SPAWN_POSITION` default (a deliberate, recorded deviation: M1-B05's/
                    // M2's own already-shipped persistence path already generalizes
                    // "spawn at a resting position" to "resume at the last-known-good
                    // one," and `join.position` already equals `SPAWN_POSITION` for a
                    // brand-new player, so this is a strict superset, never a regression).
                    // `PlayerMarker::position`/`rotation`/`on_ground` stay a synced mirror
                    // of `PlayerMotion`, kept current by this tick loop's own movement-
                    // resolution step below -- `block_action.rs`'s reach check and this
                    // same loop's own chunk-streaming/persistence steps (neither owned by
                    // this blueprint) read `PlayerMarker` directly and are not rewired.
                    region.world.spawn((
                        PlayerMarker {
                            network_entity_id: join.network_entity_id,
                            username: join.username,
                            connection: join.connection,
                            uuid: join.uuid,
                            position: join.position,
                            rotation: join.rotation,
                            on_ground: true,
                            last_streamed_center: join_chunk,
                            sent_chunks: already_sent,
                        },
                        PlayerMotion {
                            position: Vec3::new(
                                join.position[0],
                                join.position[1],
                                join.position[2],
                            ),
                            velocity: Vec3::ZERO,
                            yaw: join.rotation[0],
                            pitch: join.rotation[1],
                            on_ground: true,
                            fall_distance: 0.0,
                        },
                        TeleportState {
                            awaiting_teleport_id: None,
                            next_teleport_id: 2,
                        },
                        // M3-B03 join-drain additions (Deliverables, `world.rs`):
                        // `GameModeState{instabuild: true}` (M1-B05's own hardcoded
                        // Creative default, preserved as the real spawn value -- `#[derive
                        // (Default)]`'s own `instabuild: false` is never relied on here,
                        // `GameModeState`'s own doc comment), `HeldItem` defaulting to
                        // `Block(Stone)` (M2-B07's own exact prior fixed placement
                        // behavior, preserved as the default), `DestroyState` with its
                        // `last_sent_stage` explicitly overridden to `-1` (Deliverables:
                        // "-1 initial, via a `Default` override in the join-drain step").
                        GameModeState { instabuild: true },
                        HeldItem(HeldItemStub::Block(PlaceableBlockKind::Stone)),
                        DestroyState {
                            last_sent_stage: -1,
                            ..Default::default()
                        },
                        // M3 field-report fix (Symptom 2): `sneaking: false` at join
                        // (`PlayerInputState`'s own doc comment) -- a real client sends its
                        // own current `player_input` state again soon after joining anyway.
                        PlayerInputState::default(),
                    ));
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

                // M3-B02 Stage-3-equivalent (Context, "Which pipeline stage", step 1):
                // drain every queued movement packet since the previous tick, merging
                // per-field "last write wins" into one coalesced report per player.
                // Evaluation itself (Stage-6b-equivalent) happens later this same tick,
                // after the block-action drain-and-apply step below, per Context's own
                // exact placement instruction (physics/collision lookups need this tick's
                // own already-refreshed `ChunkIndex`, which the block-action step below
                // is itself the first consumer of).
                let mut pending_moves: std::collections::HashMap<i32, PendingMoveReport> =
                    std::collections::HashMap::new();
                for carried in std::mem::take(&mut carried_movement_updates) {
                    merge_move_report(
                        pending_moves.entry(carried.network_entity_id).or_default(),
                        &carried.report,
                    );
                }
                while let Ok(packet) = movement_rx.try_recv() {
                    merge_move_report(
                        pending_moves.entry(packet.network_entity_id).or_default(),
                        &packet.report,
                    );
                }

                // M3 field-report fix (Symptom 2): drained here, ahead of the block-action
                // drain-and-apply step below (unlike `pending_moves`, resolved later) --
                // that same step's own reach check reads `PlayerInputState` directly off each
                // acting player's own entity, so this tick's freshest sneak state must already
                // be applied before it runs. A not-yet-spawned target (the same join/action
                // mpsc-ordering race every other per-tick queue in this loop already
                // tolerates) is best-effort dropped, not carried forward -- functionally
                // harmless, since a real client resends `player_input` on every intent change
                // (and the client's own game loop keeps sending it while a movement key stays
                // held), self-correcting within the next packet or two.
                while let Ok(input) = player_input_rx.try_recv() {
                    if let Some(entity) = find_player_entity(&region.world, input.network_entity_id)
                        && let Some(mut state) = region.world.get_mut::<PlayerInputState>(entity)
                    {
                        state.sneaking = input.sneaking;
                    }
                }

                // M2 field-report chunk-streaming fix, M3 field-report re-placement (Defect
                // C): the actual crossing-detection/re-center step used to live here, reading
                // `PlayerMarker::position` before it had been refreshed for this tick -- a
                // stale, one-tick-late read (`PlayerMarker::position`'s own doc comment: kept
                // current by "this tick loop's own movement-resolution step below"). Moved
                // below, after that same movement-resolution (Stage-6b-equivalent) step has
                // written this tick's real, freshly resolved position into every currently-
                // spawned player's `PlayerMarker` -- see the relocated step's own doc comment
                // for the full rationale. `ticket_manager.step()`/`ChunkIndex` refresh/`chunk_
                // grid_requests`/`pending_stream_chunks` resolution below are unaffected by
                // this move (Context, M3-B02: they run here, before the block-action drain,
                // regardless of player-movement-driven ticket re-centering).

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

                // M3-B03's own Stage-3-equivalent manual step (Context, "Which pipeline
                // stage", step 1): drain every block action queued since the previous tick,
                // stable-sort by ascending `network_entity_id` (MECH-D4's "deterministic
                // merge by ascending player id"), reach-validate (`mining::is_within_block_
                // interaction_range`, MECH-D62 -- M3 field-report fix: a box-distance-from-eye
                // predicate, not a raycast; `mining.rs`'s own top-of-file doc comment has the
                // full retirement note), then dispatch into `mining`'s own dig-lifecycle/
                // placement functions -- each mutation immediately followed by `mining::
                // settle_neighbor_updates` (Context above) -- entirely before `executor.
                // tick_region` runs this tick's own formally-numbered pipeline (which is what
                // actually drives Stage 4 for real, now that it is wired in).
                let mut pending: Vec<PendingBlockAction> =
                    std::mem::take(&mut carried_block_actions);
                while let Ok(action) = block_action_rx.try_recv() {
                    pending.push(action);
                }
                pending.sort_by_key(|action| action.network_entity_id);

                // M3-B03: this tick's own Stage-4 resources, pulled out of `region.world` for
                // the duration of this manual step (`World::remove_resource`/`insert_resource`
                // -- the ordinary borrow checker has no way to prove a `&mut World` used for
                // `DirectBlockWorld`'s own entity mutation is disjoint from these same
                // resources also living inside that `World`, so they are held here instead,
                // as plain owned locals, and reinserted once the loop below finishes).
                let mut engine = region
                    .world
                    .remove_resource::<rc_mechanics::NeighborUpdateEngine>()
                    .expect("bootstrap_default_stage4_resources always inserts this");
                let mut scheduled = region
                    .world
                    .remove_resource::<rc_mechanics::ScheduledTickQueue>()
                    .expect("bootstrap_default_stage4_resources always inserts this");
                let mut events = region
                    .world
                    .remove_resource::<rc_mechanics::BlockEventQueue>()
                    .expect("bootstrap_default_stage4_resources always inserts this");
                let behaviors = region
                    .world
                    .remove_resource::<rc_mechanics::BlockBehaviorRegistry>()
                    .expect("bootstrap_default_stage4_resources always inserts this");
                // M2 stays inside M1-B05's single `HARDCODED_REGION_ID` (Context, still true
                // at M3's own scope -- ARCH-D24's own real directory remains a later
                // blueprint's job) -- every chunk key is trivially local by construction.
                let mining_ownership =
                    RegionOwnership::always_local(Address::Region(HARDCODED_REGION_ID));
                let mut mining_outbound: Vec<(Address, RegionMessage)> = Vec::new();
                let current_tick = region.tick_counter;

                for action in pending {
                    // The final *write* position (`resolve_place_position`'s own offset for
                    // `Place`, unchanged from block_action.rs) -- used for the chunk-
                    // residency pre-check and, inside `mining::finalize_break`/
                    // `apply_placement` themselves, for the actual mutation.
                    let write_target = target_position(&action.kind);
                    // The raw *clicked* position -- what `mining::is_within_block_interaction_
                    // range` validates against. Deliberately **not** `write_target` for a
                    // `Place` action: a `Place`'s resolved write cell is frequently still air
                    // (nothing there to have clicked in the first place), while the clicked
                    // cell a real client's own local aim actually landed on is always the same
                    // value regardless of `inside_block` -- `location` itself.
                    let reach_click = match &action.kind {
                        BlockActionKind::StartDestroy { location }
                        | BlockActionKind::StopDestroy { location }
                        | BlockActionKind::AbortDestroy { location } => Some(*location),
                        BlockActionKind::Place { location, .. } => Some(*location),
                        BlockActionKind::Ignored => None,
                    };

                    let entity = find_player_entity(&region.world, action.network_entity_id);
                    // M2 field-report fix, restated for the raycast era: falls back to a
                    // synthetic motion at `SPAWN_POSITION` for the same join/action mpsc-
                    // ordering race the original fix handled -- never panics on a
                    // not-yet-spawned actor. `pitch: 90.0` (straight down), not `0.0`: reach
                    // itself no longer depends on look direction at all (M3 field-report fix,
                    // `mining::is_within_block_interaction_range`), but `motion.yaw`/`pitch`
                    // still feed `mining::apply_placement`'s own orientation resolution
                    // (`resolve_orientation`) for this same not-yet-spawned-actor fallback --
                    // straight down is kept as the least-arbitrary default for that purpose.
                    let motion = entity
                        .and_then(|e| region.world.get::<PlayerMotion>(e))
                        .cloned()
                        .unwrap_or_else(|| PlayerMotion {
                            position: Vec3::new(
                                SPAWN_POSITION.x as f64,
                                SPAWN_POSITION.y as f64,
                                SPAWN_POSITION.z as f64,
                            ),
                            velocity: Vec3::ZERO,
                            yaw: 0.0,
                            pitch: 90.0,
                            on_ground: true,
                            fall_distance: 0.0,
                        });
                    let instabuild = entity
                        .and_then(|e| region.world.get::<GameModeState>(e))
                        .map(|g| g.instabuild)
                        .unwrap_or(true);
                    let held = entity
                        .and_then(|e| region.world.get::<HeldItem>(e))
                        .map(|h| h.0)
                        .unwrap_or(HeldItemStub::Block(PlaceableBlockKind::Stone));
                    let mut destroy_state = entity
                        .and_then(|e| region.world.get::<DestroyState>(e))
                        .copied()
                        .unwrap_or_default();
                    // M3 field-report fix (Symptom 2): pose for this action's own eye height
                    // -- `PlayerInputState`'s own doc comment has the full "no flying state
                    // tracked" caveat this crouching-iff-shift reduction relies on. A
                    // not-yet-spawned actor (no `PlayerInputState` component yet) falls back
                    // to standing, matching every other per-player fallback in this block.
                    let crouching = entity
                        .and_then(|e| region.world.get::<PlayerInputState>(e))
                        .map(|s| s.sneaking)
                        .unwrap_or(false);

                    let range = if instabuild {
                        BLOCK_INTERACTION_RANGE_CREATIVE
                    } else {
                        BLOCK_INTERACTION_RANGE_SURVIVAL
                    };
                    let in_reach = match reach_click {
                        Some(claimed) => {
                            let eye = eye_position(motion.position, crouching);
                            mining::is_within_block_interaction_range(eye, claimed, range)
                        }
                        None => true,
                    };

                    // Only a reach-validated, targeted action needs its target chunk
                    // resident before it can be applied -- an `Ignored` action (`write_
                    // target` is `None`) or an out-of-reach one is fully resolved without
                    // ever touching chunk data.
                    if let Some(target) = write_target
                        && in_reach
                        && !lifecycle.is_resident(target.chunk_key(DimensionId::OVERWORLD))
                    {
                        carried_block_actions.push(action);
                        continue;
                    }

                    if reach_click.is_some() && !in_reach {
                        send_ack(&action);
                        continue;
                    }

                    let tool = match held {
                        HeldItemStub::Tool(material, kind) => (material, kind),
                        HeldItemStub::Block(_) | HeldItemStub::EmptyHand => {
                            (ToolMaterial::None, ToolKind::None)
                        }
                    };

                    match action.kind {
                        BlockActionKind::StartDestroy { location } => {
                            send_ack(&action);
                            if instabuild {
                                let pre_break = read_raw_state(
                                    &region.world,
                                    region.world.resource::<ChunkIndex>(),
                                    DimensionId::OVERWORLD,
                                    location,
                                );
                                let outcome = mining::finalize_break(
                                    &mut DirectBlockWorld {
                                        world: &mut region.world,
                                        dimension: DimensionId::OVERWORLD,
                                        local: Address::Region(HARDCODED_REGION_ID),
                                    },
                                    &mut engine,
                                    &mut scheduled,
                                    &mut events,
                                    &mut mining_outbound,
                                    &mining_ownership,
                                    &behaviors,
                                    current_tick,
                                    location,
                                    true,
                                    tool,
                                );
                                respond_break(&region.world, &action, outcome, pre_break);
                            } else {
                                let props = mining::dig_properties_for_raw_state(read_raw_state(
                                    &region.world,
                                    region.world.resource::<ChunkIndex>(),
                                    DimensionId::OVERWORLD,
                                    location,
                                ));
                                let speed = mining::destroy_speed(
                                    props,
                                    tool,
                                    0,
                                    0,
                                    0,
                                    false,
                                    !motion.on_ground,
                                );
                                let dig_outcome = mining::begin_destroy(
                                    &mut destroy_state,
                                    location,
                                    false,
                                    speed,
                                    current_tick,
                                );
                                if dig_outcome == DestroyOutcome::FinalizeNow {
                                    let pre_break = read_raw_state(
                                        &region.world,
                                        region.world.resource::<ChunkIndex>(),
                                        DimensionId::OVERWORLD,
                                        location,
                                    );
                                    let outcome = mining::finalize_break(
                                        &mut DirectBlockWorld {
                                            world: &mut region.world,
                                            dimension: DimensionId::OVERWORLD,
                                            local: Address::Region(HARDCODED_REGION_ID),
                                        },
                                        &mut engine,
                                        &mut scheduled,
                                        &mut events,
                                        &mut mining_outbound,
                                        &mining_ownership,
                                        &behaviors,
                                        current_tick,
                                        location,
                                        false,
                                        tool,
                                    );
                                    respond_break(&region.world, &action, outcome, pre_break);
                                }
                            }
                        }
                        BlockActionKind::StopDestroy { location } => {
                            send_ack(&action);
                            if !instabuild {
                                let props = mining::dig_properties_for_raw_state(read_raw_state(
                                    &region.world,
                                    region.world.resource::<ChunkIndex>(),
                                    DimensionId::OVERWORLD,
                                    location,
                                ));
                                let speed = mining::destroy_speed(
                                    props,
                                    tool,
                                    0,
                                    0,
                                    0,
                                    false,
                                    !motion.on_ground,
                                );
                                let stop_outcome = mining::stop_destroy(
                                    &mut destroy_state,
                                    location,
                                    speed,
                                    current_tick,
                                );
                                if stop_outcome == StopOutcome::FinalizeNow {
                                    let pre_break = read_raw_state(
                                        &region.world,
                                        region.world.resource::<ChunkIndex>(),
                                        DimensionId::OVERWORLD,
                                        location,
                                    );
                                    let outcome = mining::finalize_break(
                                        &mut DirectBlockWorld {
                                            world: &mut region.world,
                                            dimension: DimensionId::OVERWORLD,
                                            local: Address::Region(HARDCODED_REGION_ID),
                                        },
                                        &mut engine,
                                        &mut scheduled,
                                        &mut events,
                                        &mut mining_outbound,
                                        &mining_ownership,
                                        &behaviors,
                                        current_tick,
                                        location,
                                        false,
                                        tool,
                                    );
                                    respond_break(&region.world, &action, outcome, pre_break);
                                }
                            }
                        }
                        BlockActionKind::AbortDestroy { .. } => {
                            send_ack(&action);
                            mining::abort_destroy(&mut destroy_state);
                        }
                        BlockActionKind::Place {
                            location,
                            face,
                            inside_block,
                            cursor,
                        } => {
                            send_ack(&action);
                            // M3 field-report fix (Defect 1, "a player can place a block
                            // inside their own body"): every currently-connected player's own
                            // AABB, crouch-aware height -- `mining::apply_placement`'s own
                            // `is_placement_obstructed` gate tests the placement's resolved
                            // collision shape against every one of these, the acting player's
                            // own body included (that function's own doc comment: never
                            // excluded by identity). This is this world's only entity kind
                            // (`is_placement_obstructed`'s own doc comment has the full
                            // "matches vanilla's blocks-building-is-false-by-default for
                            // everything else" boundary note), so a query over `PlayerMotion`/
                            // `PlayerInputState` is already the complete collection.
                            let player_boxes: Vec<Aabb> = {
                                let mut query =
                                    region.world.query::<(&PlayerMotion, &PlayerInputState)>();
                                query
                                    .iter(&region.world)
                                    .map(|(player_motion, input)| {
                                        let height = if input.sneaking {
                                            PLAYER_HEIGHT_SNEAKING
                                        } else {
                                            PLAYER_HEIGHT
                                        };
                                        Aabb::from_position(
                                            player_motion.position,
                                            PLAYER_HALF_WIDTH,
                                            height,
                                        )
                                    })
                                    .collect()
                            };
                            let outcome = mining::apply_placement(
                                &mut DirectBlockWorld {
                                    world: &mut region.world,
                                    dimension: DimensionId::OVERWORLD,
                                    local: Address::Region(HARDCODED_REGION_ID),
                                },
                                &mut engine,
                                &mut scheduled,
                                &mut events,
                                &mut mining_outbound,
                                &mining_ownership,
                                &behaviors,
                                current_tick,
                                location,
                                face,
                                inside_block,
                                cursor,
                                held,
                                motion.yaw,
                                motion.pitch,
                                &player_boxes,
                            );
                            respond_place(&region.world, &action, outcome);
                        }
                        BlockActionKind::Ignored => {
                            send_ack(&action);
                        }
                    }

                    if let Some(e) = entity
                        && let Some(mut stored) = region.world.get_mut::<DestroyState>(e)
                    {
                        *stored = destroy_state;
                    }
                }

                // `mining_outbound` is merged into `region.message_state` once, below, after
                // the destroy-state tick substep has had its own chance to append to it too
                // (a finalized delayed destroy's own `finalize_break` call also writes into
                // this same buffer) -- one merge, not two, so emission order stays a single
                // well-defined sequence.

                // M3-B03's own destroy-state tick substep (Context, "Which pipeline stage",
                // step 2): for every player currently in the region, recompute/rebroadcast
                // crack-stage progress or finalize a delayed destroy -- exactly mirroring
                // vanilla's own real "tick() runs once per player, after that tick's
                // packets" ordering. Two passes for the same borrow-checker reason the
                // movement-evaluation step below uses one: the first collects each player's
                // own current `DestroyState`/held tool/network id behind only immutable
                // borrows, the second applies mutations and sends packets.
                #[allow(clippy::type_complexity)]
                let mut destroy_tick_subjects: Vec<(
                    Entity,
                    i32,
                    DestroyState,
                    (ToolMaterial, ToolKind),
                )> = Vec::new();
                {
                    let mut query = region
                        .world
                        .query::<(Entity, &PlayerMarker, &DestroyState, &HeldItem)>();
                    for (entity, marker, state, held) in query.iter(&region.world) {
                        if !state.is_destroying && !state.has_delayed_destroy {
                            continue;
                        }
                        let tool = match held.0 {
                            HeldItemStub::Tool(material, kind) => (material, kind),
                            HeldItemStub::Block(_) | HeldItemStub::EmptyHand => {
                                (ToolMaterial::None, ToolKind::None)
                            }
                        };
                        destroy_tick_subjects.push((
                            entity,
                            marker.network_entity_id,
                            *state,
                            tool,
                        ));
                    }
                }
                for (entity, network_entity_id, mut state, tool) in destroy_tick_subjects {
                    let air = to_storage_id(AIR.0);
                    let index = region.world.resource::<ChunkIndex>();
                    let props_at_pos = mining::dig_properties_for_raw_state(read_raw_state(
                        &region.world,
                        index,
                        DimensionId::OVERWORLD,
                        state.destroy_pos,
                    ));
                    let current_at_pos = to_storage_id(read_raw_state(
                        &region.world,
                        region.world.resource::<ChunkIndex>(),
                        DimensionId::OVERWORLD,
                        state.destroy_pos,
                    ));
                    let current_at_delayed = to_storage_id(read_raw_state(
                        &region.world,
                        region.world.resource::<ChunkIndex>(),
                        DimensionId::OVERWORLD,
                        state.delayed_destroy_pos,
                    ));
                    let speed = mining::destroy_speed(props_at_pos, tool, 0, 0, 0, false, false);
                    let tick_outcome = mining::tick_destroy_state(
                        &mut state,
                        speed,
                        current_tick,
                        current_at_pos,
                        current_at_delayed,
                        air,
                    );
                    match tick_outcome {
                        TickOutcome::ActiveProgress(stage) => {
                            let stage = stage as i8;
                            if stage != state.last_sent_stage {
                                state.last_sent_stage = stage;
                                let payload = encode_payload(&SetBlockDestroyStage {
                                    entity_id: network_entity_id,
                                    location: pack_position(state.destroy_pos),
                                    destroy_stage: stage,
                                });
                                broadcast_to_others(&region.world, network_entity_id, payload);
                            }
                        }
                        TickOutcome::FinalizeDelayedNow => {
                            let pos = state.delayed_destroy_pos;
                            let pre_break = read_raw_state(
                                &region.world,
                                region.world.resource::<ChunkIndex>(),
                                DimensionId::OVERWORLD,
                                pos,
                            );
                            let outcome = mining::finalize_break(
                                &mut DirectBlockWorld {
                                    world: &mut region.world,
                                    dimension: DimensionId::OVERWORLD,
                                    local: Address::Region(HARDCODED_REGION_ID),
                                },
                                &mut engine,
                                &mut scheduled,
                                &mut events,
                                &mut mining_outbound,
                                &mining_ownership,
                                &behaviors,
                                current_tick,
                                pos,
                                false,
                                tool,
                            );
                            if let BreakOutcome::Applied { .. } = outcome {
                                broadcast_break(
                                    &region.world,
                                    pos,
                                    AIR.0,
                                    pre_break,
                                    network_entity_id,
                                );
                            }
                        }
                        TickOutcome::Idle
                        | TickOutcome::CancelledBlockChanged
                        | TickOutcome::CancelledDelayedBlockChanged => {}
                    }
                    if let Some(mut stored) = region.world.get_mut::<DestroyState>(entity) {
                        *stored = state;
                    }
                }
                if !mining_outbound.is_empty() {
                    let mut bus = RegionMessageBus::new();
                    for (to, msg) in mining_outbound {
                        bus.send(to, msg);
                    }
                    region.message_state.merge(bus);
                }

                region.world.insert_resource(engine);
                region.world.insert_resource(scheduled);
                region.world.insert_resource(events);
                region.world.insert_resource(behaviors);
                // M3-B03 (Context, "Wiring M3-B01's Stage-4 substrate"): keeps `stage4::
                // ecs::ChunkIndex` (a *different* resource type from `block_action::
                // ChunkIndex`, despite the identical name -- distinct crates) current too,
                // so a future sibling blueprint's own real `BlockBehavior`s (registered into
                // the same `BlockBehaviorRegistry` this tick already threaded through) see
                // accurate chunk residency once `executor.tick_region` below actually runs
                // Stage 4's own two systems this same tick.
                {
                    let mut stage4_index = rc_mechanics::stage4::ecs::ChunkIndex::default();
                    let mut chunk_query = region.world.query::<(&ChunkKeyTag, Entity)>();
                    for (tag, entity) in chunk_query.iter(&region.world) {
                        stage4_index.0.insert(tag.0, entity);
                    }
                    region.world.insert_resource(stage4_index);
                }

                // M3-B02 Stage-6b-equivalent (Context, "Which pipeline stage", step 2):
                // evaluates every player CURRENTLY in the region (not just those with a
                // fresh report this tick -- Context's own "gravity/collision bookkeeping
                // for a player who sent no packet this tick is a documented no-op, not a
                // bug" rule), placed here -- after the block-action drain-and-apply step
                // above, before `executor.tick_region` below -- per Context's own exact
                // instruction: this tick's `ChunkIndex` refresh (above) is what
                // `ChunkBlockShapeSource`'s block-shape lookups need already current, and
                // the block-action step above is itself the first consumer of that same
                // refresh, so movement evaluation follows it rather than the reverse.
                //
                // Two passes to satisfy the borrow checker: the first collects each
                // player's evaluated outcome behind only immutable borrows of
                // `region.world` (`ChunkBlockShapeSource` and the query both borrow it
                // shared, so they may coexist); the second, once those borrows have
                // ended, writes the results back and sends responses.
                #[allow(clippy::type_complexity)]
                let mut move_results: Vec<(
                    Entity,
                    ConnectionHandle,
                    uuid::Uuid,
                    PlayerMotion,
                    TeleportState,
                    MovementOutcome,
                    bool,
                )> = Vec::new();
                {
                    // `World::query` itself needs `&mut World` momentarily (to register the
                    // query's component access) even though the returned `QueryState` does
                    // not borrow `world` at all -- built before `shapes` below so that
                    // momentary mutable borrow never overlaps `shapes`'s own immutable one.
                    let mut query =
                        region
                            .world
                            .query::<(Entity, &PlayerMarker, &PlayerMotion, &TeleportState)>();
                    let shapes = ChunkBlockShapeSource {
                        world: &region.world,
                        index: region.world.resource::<ChunkIndex>(),
                        dimension: DimensionId::OVERWORLD,
                    };
                    let mut entries: Vec<(
                        Entity,
                        i32,
                        ConnectionHandle,
                        uuid::Uuid,
                        PlayerMotion,
                        TeleportState,
                    )> = query
                        .iter(&region.world)
                        .map(|(entity, marker, motion, teleport)| {
                            (
                                entity,
                                marker.network_entity_id,
                                marker.connection.clone(),
                                marker.uuid,
                                motion.clone(),
                                teleport.clone(),
                            )
                        })
                        .collect();
                    entries.sort_by_key(|(_, network_id, ..)| *network_id);

                    for (entity, network_id, connection, uuid, mut motion, mut teleport) in entries
                    {
                        let report = pending_moves.remove(&network_id);
                        let had_report = report.is_some();
                        let report = report.unwrap_or_default();
                        let outcome =
                            evaluate_movement(&mut motion, &mut teleport, &report, &shapes);
                        move_results.push((
                            entity, connection, uuid, motion, teleport, outcome, had_report,
                        ));
                    }
                }

                for (entity, connection, uuid, motion, teleport, outcome, had_report) in
                    move_results
                {
                    if let Some(mut stored) = region.world.get_mut::<PlayerMotion>(entity) {
                        *stored = motion.clone();
                    }
                    if let Some(mut stored) = region.world.get_mut::<TeleportState>(entity) {
                        *stored = teleport.clone();
                    }
                    if let Some(mut marker) = region.world.get_mut::<PlayerMarker>(entity) {
                        // Mirrors `motion`'s own resolved fields back onto `PlayerMarker`
                        // (Context: not this blueprint's own state, but `block_action.rs`'s
                        // reach check and this same loop's own chunk-streaming/persistence
                        // steps -- neither owned by this blueprint -- still read it
                        // directly, per the join-drain step's own doc comment above).
                        marker.position = [motion.position.x, motion.position.y, motion.position.z];
                        marker.rotation = [motion.yaw, motion.pitch];
                        marker.on_ground = motion.on_ground;
                    }
                    if had_report {
                        // M2 field-report persistence fix, preserved: syncs the live
                        // position/rotation straight back into this player's own session
                        // record whenever a fresh report actually arrived this tick, not
                        // only at disconnect -- `sessions_for_thread.save_all()`'s own
                        // periodic sweep (below) and `SaveOnDisconnect`'s own
                        // disconnect-time save (`connection.rs`) both simply persist
                        // whatever this record currently holds. AC1c's own "player rejoins
                        // at the position they left at" fix.
                        let position = [motion.position.x, motion.position.y, motion.position.z];
                        let rotation = [motion.yaw, motion.pitch];
                        sessions_for_thread.with_record_mut(uuid, |record| {
                            record.data.pos = position;
                            record.data.rotation = rotation;
                        });
                    }
                    respond_to_movement(&connection, &motion, &teleport, outcome);
                }

                // M2 field-report chunk-streaming fix, M3 field-report re-placement (Defect
                // C -- corrects a one-tick-stale read): recomputes every currently-spawned
                // player's own current chunk and, for any whose chunk changed since the
                // last time this ran, re-centers their ticket (`TicketManager::move_player`
                // -- that method's own doc comment: "no production call site exists at M2...
                // exposed for a future mechanics blueprint," this is that call site now),
                // sends the matching `SetChunkCacheCenter` update (cadence: only on an
                // actual chunk-key change, `PlayerMarker::last_streamed_center`'s own doc
                // comment -- matches vanilla's own `ClientboundSetChunkCacheCenterPacket`
                // cadence), and queues every newly-visible, not-yet-sent chunk coordinate
                // for a later tick's own streaming resolution step (the one above, ahead of
                // the block-action drain -- Context, M3-B02, kept there since it needs no
                // fresh position of its own, only this tick's or an earlier tick's already-
                // queued `pending_stream_chunks`/`chunk_grid_requests`). Runs every tick
                // unconditionally -- cheap (`O(players)`) and a correct no-op whenever
                // nobody's chunk changed. This is AC1b's own "walking N chunks streams new
                // chunks in" fix.
                //
                // Placed here, after the movement-resolution loop directly above has written
                // this tick's real, freshly resolved position into every currently-spawned
                // player's `PlayerMarker` (the `marker.position = ...` assignment in that
                // same loop) -- not at the top of the tick, where `PlayerMarker::position`
                // still held the *previous* tick's resolved value the whole time this step's
                // own crossing check ran (M3 field-report Defect C: every chunk-boundary
                // crossing was therefore acted on one tick, ~50 ms, late -- a latency bug,
                // not data loss, since it self-corrected every following tick regardless).
                // `PlayerMotion`/`PlayerMarker` deliberately stay two separate components
                // (Context, `PlayerMarker::position`'s own doc comment) so this step keeps
                // reading the mirror, now simply read at the point in the tick where that
                // mirror is actually current. Movement resolution itself stays exactly where
                // M3-B02 placed it -- after the block-action drain, so its own collision
                // lookups see this tick's already-refreshed `ChunkIndex` -- this fix only
                // reorders this streaming step relative to that fixed point, never movement
                // resolution relative to the block-action drain.
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

                // Any report left over belongs to a `network_entity_id` with no
                // `PlayerMarker` spawned in `region.world` yet this tick (the same
                // join/action mpsc-ordering race `broadcast_to_all`'s own doc comment
                // handles for block actions) -- carried into the next tick's own drain
                // rather than dropped.
                for (network_entity_id, report) in pending_moves {
                    carried_movement_updates.push(PendingMovementPacket {
                        network_entity_id,
                        report,
                    });
                }

                // M3-B03, test/diagnostic only: applies every queued `debug_set_held_item`/
                // `debug_set_survival` call against this tick's own currently-spawned
                // players -- a not-yet-spawned target (the same join/action mpsc-ordering
                // race every other queue in this loop already tolerates) is silently
                // dropped (its own oneshot reply simply goes unsent, which the caller's own
                // `.await` on the receiving end observes as an error rather than hanging
                // forever) rather than carried forward, matching this pair's own "test/
                // diagnostic only" scope. Each call carries its own oneshot reply, sent only
                // once the mutation has actually been applied -- `debug_set_held_item`/
                // `debug_set_survival` are themselves `async fn`s the caller awaits, a
                // deliberate deviation from this blueprint's own literal synchronous
                // signature (Deliverables): a fire-and-forget send racing an unrelated
                // round trip on a *different* channel (this file's own earlier draft) is not
                // actually a reliable synchronization primitive -- two independently-polled
                // channels give no guarantee about which one a given tick iteration observes
                // first, even when the sends themselves are strictly ordered, so a test built
                // on that assumption is flaky exactly the way a real CI run under heavy
                // parallel load caught directly, not hypothetically.
                while let Ok((network_entity_id, item, ack)) = debug_held_item_rx.try_recv() {
                    if let Some(entity) = find_player_entity(&region.world, network_entity_id)
                        && let Some(mut held) = region.world.get_mut::<HeldItem>(entity)
                    {
                        held.0 = item;
                    }
                    let _ = ack.send(());
                }
                while let Ok((network_entity_id, survival, ack)) = debug_survival_rx.try_recv() {
                    if let Some(entity) = find_player_entity(&region.world, network_entity_id)
                        && let Some(mut mode) = region.world.get_mut::<GameModeState>(entity)
                    {
                        mode.instabuild = !survival;
                    }
                    let _ = ack.send(());
                }

                while let Ok((pos, reply)) = query_rx.try_recv() {
                    let _ = reply.send(debug_query_block(
                        &region.world,
                        DimensionId::OVERWORLD,
                        pos,
                    ));
                }

                while let Ok(reply) = stage4_counters_rx.try_recv() {
                    let engine = region
                        .world
                        .resource::<rc_mechanics::NeighborUpdateEngine>();
                    let scheduled = region.world.resource::<rc_mechanics::ScheduledTickQueue>();
                    let events = region.world.resource::<rc_mechanics::BlockEventQueue>();
                    let _ = reply.send(Stage4Counters {
                        neighbor_engine_idle: engine.is_idle(),
                        block_ticks_pending: scheduled.block_len(),
                        fluid_ticks_pending: scheduled.fluid_len(),
                        block_events_pending_next_tick: events.pending_next_tick(),
                    });
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
            player_input_tx,
            query_tx,
            next_network_entity_id: Arc::new(AtomicI32::new(1)),
            shutdown_flag,
            thread_handle: Arc::new(Mutex::new(Some(handle))),
            chunk_grid_tx,
            sessions,
            debug_held_item_tx,
            debug_survival_tx,
            stage4_counters_tx,
        }
    }

    /// Allocates the next network-facing entity id (starts at `1`, monotonic, thread-safe).
    pub fn alloc_network_entity_id(&self) -> i32 {
        self.next_network_entity_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Enqueues a `PlayerMarker` spawn, applied at the start of the region's next tick
    /// (Context's join-queue). Never blocks (`UnboundedSender::send` never blocks).
    ///
    /// `Err(RegionUnavailable)` iff the hardcoded region's tick-loop thread has already
    /// died (M3 field-report fix, symptom 2: this project's single hardcoded region has no
    /// supervision/restart, so once that thread is gone it stays gone for the rest of the
    /// process's life) -- every caller treats this the same way `try_send_payload`
    /// failures elsewhere in `connection.rs` are already treated: close/refuse this one
    /// connection attempt with a diagnostic, never panic the per-connection task over it.
    pub fn queue_join(&self, join: PendingJoin) -> Result<(), RegionUnavailable> {
        self.join_tx.send(join).map_err(|_| RegionUnavailable)
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
    /// tick's Stage-3-equivalent step (Context). Never blocks. `queue_join`'s own doc
    /// comment has the full `Err` rationale, shared by every method below.
    pub fn queue_block_action(&self, action: PendingBlockAction) -> Result<(), RegionUnavailable> {
        self.block_action_tx
            .send(action)
            .map_err(|_| RegionUnavailable)
    }

    /// M3-B02 (superseding the M2 field-report movement-application fix's own `queue_
    /// movement`). Enqueues a decoded movement packet, applied at the start of this
    /// region's next tick's Stage-3-equivalent step (Context). Never blocks.
    pub fn queue_movement_packet(
        &self,
        packet: PendingMovementPacket,
    ) -> Result<(), RegionUnavailable> {
        self.movement_tx.send(packet).map_err(|_| RegionUnavailable)
    }

    /// M3 field-report fix (Symptom 2). Enqueues a decoded `player_input` packet, applied
    /// early in this region's next tick -- ahead of that same tick's own block-action
    /// drain-and-apply step, so a reach check later in the same tick already sees the latest
    /// sneak state (Context). Never blocks.
    pub fn queue_player_input(&self, input: PendingPlayerInput) -> Result<(), RegionUnavailable> {
        self.player_input_tx
            .send(input)
            .map_err(|_| RegionUnavailable)
    }

    /// New, test/diagnostic only (Context, `debug_query_block`'s own doc comment). Awaits
    /// this tick's or the next tick's debug-query drain step, whichever comes first after
    /// the call. Already `Option`-returning for "not found" -- a dead tick-loop thread (send
    /// failure, or the reply sender dropped mid-flight) folds into that same `None`, exactly
    /// as `queue_join`'s own doc comment describes, without needing a distinct error type
    /// for a method whose contract was already "maybe nothing" (`block_action.rs`'s doc
    /// comment on this same return shape).
    pub async fn debug_query_block(&self, pos: BlockPos) -> Option<DebugBlockInfo> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.query_tx.send((pos, reply_tx)).ok()?;
        reply_rx.await.ok().flatten()
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
    ///
    /// `None` iff the tick-loop thread is already gone (`queue_join`'s own doc comment) --
    /// this is `enter_play`'s own very first fallible call into `HardcodedWorld`, made
    /// before any player state exists to clean up, so a plain "stop joining" is enough for
    /// the caller (`connection.rs`).
    pub async fn request_chunk_grid(
        &self,
        network_entity_id: i32,
        center: ChunkKey,
        ticket_radius: u8,
        coords: Vec<(i32, i32)>,
    ) -> Option<Vec<Vec<u8>>> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.chunk_grid_tx
            .send(ChunkGridRequest {
                network_entity_id,
                center,
                ticket_radius,
                coords,
                reply: reply_tx,
            })
            .ok()?;
        reply_rx.await.ok()
    }

    /// New (M2 integration, M2-B06's own "Composition-root integration" recipe step 2/3):
    /// exposes this world's player-record working set to `enter_play`. `Clone`, cheap
    /// (`Arc`-backed), matching every other `HardcodedWorld` handle method's own shape.
    pub fn player_sessions(&self) -> PlayerSessionStore {
        self.sessions.clone()
    }

    /// Test/diagnostic only (Context: mirrors `debug_query_block`'s own precedent).
    /// Applied at the start of the region's next tick that finds `network_entity_id`
    /// currently spawned; `.await`s that same tick's own oneshot confirmation that the
    /// mutation was applied (or silently dropped, if `network_entity_id` was never found)
    /// before returning -- a deliberate deviation from this blueprint's own literal
    /// synchronous signature (Deliverables), recorded here and in the completion report:
    /// see `world.rs`'s own tick-loop drain doc comment for why a fire-and-forget send here
    /// is not actually a reliable way for a caller to know the mutation has landed.
    pub async fn debug_set_held_item(&self, network_entity_id: i32, item: HeldItemStub) {
        let (ack_tx, ack_rx) = oneshot::channel();
        if self
            .debug_held_item_tx
            .send((network_entity_id, item, ack_tx))
            .is_ok()
        {
            let _ = ack_rx.await;
        }
    }

    /// As `debug_set_held_item`. `survival: true` sets `GameModeState.instabuild = false`
    /// (Context: "the smallest possible slice of MECH-D60's abilities model needed").
    pub async fn debug_set_survival(&self, network_entity_id: i32, survival: bool) {
        let (ack_tx, ack_rx) = oneshot::channel();
        if self
            .debug_survival_tx
            .send((network_entity_id, survival, ack_tx))
            .is_ok()
        {
            let _ = ack_rx.await;
        }
    }

    /// Test/diagnostic only -- reads `NeighborUpdateEngine::is_idle()`/`ScheduledTickQueue::
    /// block_len()`/`fluid_len()`/`BlockEventQueue::pending_next_tick()` straight off
    /// `region.world`'s own M3-B01 resources (Acceptance tests, `mining_stage4_wiring.rs`).
    /// Awaits the next tick's drain, mirroring `debug_query_block`.
    pub async fn debug_stage4_counters(&self) -> Stage4Counters {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.stage4_counters_tx
            .send(reply_tx)
            .expect("the hardcoded region's tick-loop thread outlives every connection");
        reply_rx.await.expect(
            "the hardcoded region's tick-loop thread always replies before dropping the sender",
        )
    }
}

/// Test/diagnostic introspection only (`debug_stage4_counters`'s own doc comment) -- the
/// M3-B01 Stage-4 substrate's own steady-state idleness, read directly off `region.world`'s
/// resources rather than inferred from wire traffic.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Stage4Counters {
    pub neighbor_engine_idle: bool,
    pub block_ticks_pending: usize,
    pub fluid_ticks_pending: usize,
    pub block_events_pending_next_tick: usize,
}

/// The raw block-state id currently stored at `pos`, or `AIR` if `pos`'s chunk is not
/// resident in `index` at all (M3-B03: shared by every dig-timing/finalize call site that
/// needs to read a world position's own current content without going through a full
/// `BlockWorldAccess` adapter -- `DirectBlockWorld`'s own `get_block` does the identical
/// lookup for the `&mut`-borrowing call sites; this is the `&`-only twin for read-only
/// lookups that must coexist with an unrelated mutable borrow of `region.world` elsewhere in
/// the same statement).
fn read_raw_state(world: &World, index: &ChunkIndex, dimension: DimensionId, pos: BlockPos) -> u32 {
    let key = pos.chunk_key(dimension);
    index
        .0
        .get(&key)
        .and_then(|&entity| world.get::<BlockStateColumn>(entity))
        .map(|column| {
            let (lx, lz) = (pos.x.rem_euclid(16) as u8, pos.z.rem_euclid(16) as u8);
            column.get(lx, pos.y, lz).to_raw()
        })
        .unwrap_or(AIR.0)
}

/// Sends exactly one `Acknowledge Block Change` to the acting connection (MECH-D63 --
/// unconditional, whether the action succeeds, is rejected, or is `Ignored`).
fn send_ack(action: &PendingBlockAction) {
    let _ = action
        .connection
        .try_send_payload(encode_payload(&AcknowledgeBlockChange {
            sequence: action.sequence,
        }));
}

/// Broadcasts `payload` to every currently-connected player, guaranteeing the actor is
/// reached even if their own `PlayerMarker` has not been spawned into `world` yet this same
/// tick (M2 field-report fix, `task_9ce21947`, restated: two independent mpsc channels
/// (`HardcodedWorld::join_tx`/`block_action_tx`) race, with no guarantee a join enqueued
/// moments before this same action's own packet has already been drained). `actor_reached`
/// tracks whether the iteration below already found and sent to the actor's own connection;
/// if not, it is sent once more directly via `actor_connection` -- never double-sent when
/// their `PlayerMarker` already existed.
fn broadcast_to_all(
    world: &World,
    actor_connection: &ConnectionHandle,
    actor_network_id: i32,
    payload: bytes::Bytes,
) {
    let mut actor_reached = false;
    for entity_ref in world.iter_entities() {
        if let Some(marker) = entity_ref.get::<PlayerMarker>() {
            let _ = marker.connection.try_send_payload(payload.clone());
            if marker.network_entity_id == actor_network_id {
                actor_reached = true;
            }
        }
    }
    if !actor_reached {
        let _ = actor_connection.try_send_payload(payload);
    }
}

/// As `broadcast_to_all`, excluding `exclude_network_id` entirely (Context: `Set Block
/// Destroy Stage`'s own "every *other* currently-connected player" rule -- the digging
/// player's own client already predicts the crack overlay locally). No "actor may not be
/// spawned yet" fallback is needed here: the excluded player is never the intended
/// recipient in the first place.
fn broadcast_to_others(world: &World, exclude_network_id: i32, payload: bytes::Bytes) {
    for entity_ref in world.iter_entities() {
        if let Some(marker) = entity_ref.get::<PlayerMarker>()
            && marker.network_entity_id != exclude_network_id
        {
            let _ = marker.connection.try_send_payload(payload.clone());
        }
    }
}

/// A finalized break's own broadcast: `Block Update` (new state, always `AIR`) to every
/// connected player, unconditionally -- the block-state resync itself is never subject to any
/// exclusion (Context: "the broadcast to every connected player interest-set simplification...
/// still valid", also this milestone's own explicitly-out-of-scope "resend to both cells" item,
/// unrelated but reaffirming the same "resync is unconditional" rule). `Level Event` (the break
/// sound/particle effect, `data` = the block's own raw *pre*-break state id) goes to every
/// OTHER connected player, excluding `exclude_network_id` -- M3 field-report fix (Defect 2,
/// "the breaking player hears the block-break effect twice"): the breaking player's own client
/// already plays the effect locally as prediction, so the server's own copy would double it
/// (Context, AUTHORITATIVE RESEARCH VERDICT); `broadcast_to_others`'s own doc comment already
/// cites this identical pattern for the digging player's own crack-overlay broadcast.
fn broadcast_break(
    world: &World,
    pos: BlockPos,
    new_state: u32,
    pre_break_state: u32,
    exclude_network_id: i32,
) {
    let update = encode_payload(&BlockUpdate {
        location: pack_position(pos),
        block_state_id: new_state as i32,
    });
    for entity_ref in world.iter_entities() {
        if let Some(marker) = entity_ref.get::<PlayerMarker>() {
            let _ = marker.connection.try_send_payload(update.clone());
        }
    }
    let level_event = encode_payload(&LevelEvent {
        event_id: LEVEL_EVENT_BLOCK_BREAK,
        location: pack_position(pos),
        data: pre_break_state as i32,
        // Block-break is never one of vanilla's global-broadcast events (`LevelEvent`'s own
        // doc comment) -- always distance-limited, per `packets.rs`.
        global_event: false,
    });
    broadcast_to_others(world, exclude_network_id, level_event);
}

/// `mining::finalize_break`'s own response side: `Applied` broadcasts `Block Update` to every
/// connected player, guaranteeing the actor is reached even if not yet spawned
/// (`broadcast_to_all`), and `Level Event` to every OTHER connected player, excluding the
/// breaker (`broadcast_to_others` -- M3 field-report fix, Defect 2; see `broadcast_break`'s own
/// doc comment for the full reasoning, identical here); `Rejected` (only ever
/// `TargetAlreadyAir`, `finalize_break`'s own only rejection) sends a corrective `Block Update`
/// to the actor alone.
fn respond_break(
    world: &World,
    action: &PendingBlockAction,
    outcome: BreakOutcome,
    pre_break_state: u32,
) {
    match outcome {
        BreakOutcome::Applied { pos, .. } => {
            let update = encode_payload(&BlockUpdate {
                location: pack_position(pos),
                block_state_id: AIR.0 as i32,
            });
            broadcast_to_all(world, &action.connection, action.network_entity_id, update);
            let level_event = encode_payload(&LevelEvent {
                event_id: LEVEL_EVENT_BLOCK_BREAK,
                location: pack_position(pos),
                data: pre_break_state as i32,
                // Block-break is never one of vanilla's global-broadcast events (`LevelEvent`'s
                // own doc comment) -- always distance-limited, per `packets.rs`.
                global_event: false,
            });
            broadcast_to_others(world, action.network_entity_id, level_event);
        }
        BreakOutcome::Rejected {
            pos, current_state, ..
        } => {
            let payload = encode_payload(&BlockUpdate {
                location: pack_position(pos),
                block_state_id: current_state as i32,
            });
            let _ = action.connection.try_send_payload(payload);
        }
    }
}

/// `mining::apply_placement`'s own response side: `Applied` broadcasts `Block Update` only
/// (no `Level Event` for a placement, Context); `Rejected` sends a corrective `Block Update`
/// to the actor alone only when `current_state` is populated (never for `RejectReason::
/// OutOfReach` -- unreachable here, already filtered before dispatch -- and never for
/// `NothingToPlace`, which has nothing to correct to).
fn respond_place(world: &World, action: &PendingBlockAction, outcome: PlaceOutcome) {
    match outcome {
        PlaceOutcome::Applied { pos, new_state } => {
            let payload = encode_payload(&BlockUpdate {
                location: pack_position(pos),
                block_state_id: new_state as i32,
            });
            broadcast_to_all(world, &action.connection, action.network_entity_id, payload);
        }
        PlaceOutcome::Rejected {
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
        PlaceOutcome::Rejected {
            current_state: None,
            ..
        } => {}
    }
}

/// `evaluate_movement`'s response side (Context: "Issuing a correction"). On
/// `RejectSpeed`/`RejectMismatch`, sends a `SynchronizePlayerPosition` correction back to
/// `motion`'s own last-known-good (unchanged by a rejected outcome) position/rotation; on
/// `Disconnect`, closes the connection; every other outcome sends nothing further (an
/// accepted move is silent -- no ack, matching vanilla's own "the server says nothing when
/// it agrees" behavior).
fn respond_to_movement(
    connection: &ConnectionHandle,
    motion: &PlayerMotion,
    teleport: &TeleportState,
    outcome: MovementOutcome,
) {
    match outcome {
        MovementOutcome::RejectSpeed | MovementOutcome::RejectMismatch => {
            let teleport_id = teleport.awaiting_teleport_id.expect(
                "evaluate_movement always sets awaiting_teleport_id before returning a Reject* outcome",
            );
            let _ = connection.try_send_payload(encode_payload(&SynchronizePlayerPosition {
                teleport_id,
                x: motion.position.x,
                y: motion.position.y,
                z: motion.position.z,
                delta_x: 0.0,
                delta_y: 0.0,
                delta_z: 0.0,
                yaw: motion.yaw,
                pitch: motion.pitch,
                relative_arguments: 0x00,
            }));
        }
        MovementOutcome::Disconnect => {
            connection.close();
        }
        MovementOutcome::NoPositionClaim
        | MovementOutcome::IgnoredAwaitingTeleport
        | MovementOutcome::Accepted => {}
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

/// M3 field-report fix (symptom 1): `DirectBlockWorld`'s own bounds guard, exercised through
/// the real `mining::finalize_break`/`apply_placement` call sites a live player's break/place
/// actually goes through (`world.rs`'s own manual Stage-3-equivalent tick step) -- the
/// original crash's own exact reproduction ("the owner broke a bedrock block at the world
/// floor", Context). Unit tests (not `crates/server/tests/**`) since `DirectBlockWorld` is
/// private to this module.
#[cfg(test)]
mod direct_block_world_bounds {
    use super::*;

    fn spawn_one_chunk(world: &mut World, filled: rc_chunk_storage::BlockStateId) -> ChunkKey {
        world.insert_resource(ChunkIndex::default());
        let key = ChunkKey::new(DimensionId::OVERWORLD, 0, 0);
        let column = BlockStateColumn::new(filled, PaletteThresholds::blocks(8));
        let entity = world
            .spawn((ChunkKeyTag(key), column, ChunkPersistenceState::new()))
            .id();
        world.resource_mut::<ChunkIndex>().0.insert(key, entity);
        key
    }

    #[test]
    fn breaking_bedrock_at_the_world_floor_does_not_panic_and_skips_the_below_world_neighbour() {
        let mut ecs_world = World::new();
        spawn_one_chunk(&mut ecs_world, to_storage_id(AIR.0));
        let floor_pos = BlockPos::new(0, WORLD_MIN_Y, 0);
        {
            let mut direct = DirectBlockWorld {
                world: &mut ecs_world,
                dimension: DimensionId::OVERWORLD,
                local: Address::Region(RegionId(1)),
            };
            assert!(direct.set_block(floor_pos, to_storage_id(BEDROCK.0)));
        }

        let ownership = RegionOwnership::always_local(Address::Region(RegionId(1)));
        let mut engine = rc_mechanics::NeighborUpdateEngine::new();
        let mut scheduled = rc_mechanics::ScheduledTickQueue::new();
        let mut events = rc_mechanics::BlockEventQueue::new();
        let mut outbound = Vec::new();
        let behaviors = rc_mechanics::BlockBehaviorRegistry::new();
        let mut direct = DirectBlockWorld {
            world: &mut ecs_world,
            dimension: DimensionId::OVERWORLD,
            local: Address::Region(RegionId(1)),
        };

        // The original crash's own exact shape: breaking the block sitting at the world's
        // own floor fans a `NeighborChanged`/`ShapeUpdate` pair out to `floor_pos`'s own
        // `Down` neighbour, one below the world -- `column.rs`'s own `section_index_for_y`
        // `assert!` before this fix, `DirectBlockWorld`'s own guard (`y_in_world_bounds`)
        // after it.
        let outcome = mining::finalize_break(
            &mut direct,
            &mut engine,
            &mut scheduled,
            &mut events,
            &mut outbound,
            &ownership,
            &behaviors,
            0,
            floor_pos,
            true,
            (ToolMaterial::None, ToolKind::None),
        );
        assert!(
            matches!(outcome, BreakOutcome::Applied { .. }),
            "{outcome:?}"
        );
        assert_eq!(direct.get_block(floor_pos), Some(to_storage_id(AIR.0)));
        // Vanilla parity (Context (c)): the below-world neighbour was never touched -- it
        // still resolves to `None`, not some spuriously-written value.
        assert_eq!(direct.get_block(BlockPos::new(0, WORLD_MIN_Y - 1, 0)), None);
    }

    #[test]
    fn placing_at_the_world_ceiling_does_not_panic_and_skips_the_above_world_neighbour() {
        let mut ecs_world = World::new();
        spawn_one_chunk(&mut ecs_world, to_storage_id(AIR.0));
        let ceiling_pos = BlockPos::new(0, WORLD_MIN_Y + WORLD_HEIGHT - 1, 0);

        let ownership = RegionOwnership::always_local(Address::Region(RegionId(1)));
        let mut engine = rc_mechanics::NeighborUpdateEngine::new();
        let mut scheduled = rc_mechanics::ScheduledTickQueue::new();
        let mut events = rc_mechanics::BlockEventQueue::new();
        let mut outbound = Vec::new();
        let behaviors = rc_mechanics::BlockBehaviorRegistry::new();
        let mut direct = DirectBlockWorld {
            world: &mut ecs_world,
            dimension: DimensionId::OVERWORLD,
            local: Address::Region(RegionId(1)),
        };

        // `inside_block: true` places directly at `location` (`resolve_place_position`'s own
        // rule) -- the world's own top valid layer, already air (the fixture's own default
        // fill) -- fanning a pair out to the `Up` neighbour, one above the world.
        let outcome = mining::apply_placement(
            &mut direct,
            &mut engine,
            &mut scheduled,
            &mut events,
            &mut outbound,
            &ownership,
            &behaviors,
            0,
            ceiling_pos,
            super::super::block_action::Face::Up,
            true,
            (0.5, 0.5, 0.5),
            HeldItemStub::Block(PlaceableBlockKind::Stone),
            0.0,
            0.0,
            // No player entities in this fixture (Defect 1's own gate is exercised by its
            // dedicated regression suite instead) -- an empty slice can never obstruct.
            &[],
        );
        assert!(
            matches!(outcome, PlaceOutcome::Applied { .. }),
            "{outcome:?}"
        );
        assert_ne!(direct.get_block(ceiling_pos), Some(to_storage_id(AIR.0)));
        assert_eq!(
            direct.get_block(BlockPos::new(0, WORLD_MIN_Y + WORLD_HEIGHT, 0)),
            None
        );
    }
}
