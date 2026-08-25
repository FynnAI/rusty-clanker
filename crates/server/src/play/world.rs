//! The one hardcoded region and its 20 TPS tick loop -- this blueprint's own composition-
//! root wiring (M1-B05 blueprint Context, "The hardcoded region and its 20 TPS tick loop").
//! No `rc_scheduler::RegionManager` -- a single region that never splits or merges has no
//! use for its merge/split lifecycle; `RcExecutor::spawn_region` is called directly.

use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

use bevy_ecs::prelude::*;
use rc_messaging::RegionId;
use rc_scheduler::RcExecutorBuilder;
use rc_scheduler::pool::{RcWorkerPool, SystemTickWaiter, TickClock};
use rc_transport_inproc::{InProcessTransport, InProcessTransportConfig};

use super::{PlayerProfile, enter_play};
use crate::net::{PlayerSession, PlayerSessionSink};

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
}

pub struct PendingJoin {
    pub network_entity_id: i32,
    pub username: String,
}

/// Owns the one hardcoded region's tick loop (its own dedicated OS thread, ARCH-D21) and a
/// network-entity-id counter, independent of `rc_core::RcEntityIdAllocator` (Context --
/// vanilla's own wire `entity_id` is a separate, small `i32` space). `Clone`, cheap (an
/// `Arc`-backed sender handle).
#[derive(Clone)]
pub struct HardcodedWorld {
    join_tx: tokio::sync::mpsc::UnboundedSender<PendingJoin>,
    next_network_entity_id: Arc<AtomicI32>,
}

impl HardcodedWorld {
    /// Spawns the tick-loop thread (Context's pseudocode) and returns a handle. The thread
    /// runs for the process lifetime; there is no shutdown API in this blueprint's scope.
    pub fn new() -> Self {
        let (join_tx, mut join_rx) = tokio::sync::mpsc::unbounded_channel::<PendingJoin>();

        std::thread::spawn(move || {
            let executor = RcExecutorBuilder::new(|_world| {})
                .build()
                .expect("zero systems never violates ARCH-D8's structural-write check");
            let mut region = executor.spawn_region(HARDCODED_REGION_ID);
            let transport = InProcessTransport::new(InProcessTransportConfig::default());
            transport.register_region(HARDCODED_REGION_ID);
            let pool = RcWorkerPool::new(4);
            let mut clock = TickClock::<SystemTickWaiter>::new();
            loop {
                while let Ok(join) = join_rx.try_recv() {
                    region.world.spawn(PlayerMarker {
                        network_entity_id: join.network_entity_id,
                        username: join.username,
                    });
                }
                executor.tick_region(&mut region, &pool, &transport);
                clock.await_next_tick();
            }
        });

        Self {
            join_tx,
            next_network_entity_id: Arc::new(AtomicI32::new(1)),
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
