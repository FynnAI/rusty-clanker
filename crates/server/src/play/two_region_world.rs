//! An additive, parallel composition to `HardcodedWorld` (M1-B05) — never modifies it.
//! Two real, simultaneously-live, independently-ticking regions with a static
//! chunk-ownership boundary at `x = 0`, sharing one `InProcessTransport` and one
//! `RcExecutor` (built once, `spawn_region`'d twice). Exists to make M4-B08's own
//! acceptance criteria genuinely exercisable, and reusable by any future blueprint that
//! needs a real multi-region test/dev harness (M4-B08 Context, Deliverables).
//!
//! **Deviations from the blueprint's own abbreviated Deliverables signatures, documented
//! once here rather than at every call site**: every `debug_*`/`queue_join`/
//! `debug_query_player_position` method below is `async` and, where the blueprint's own
//! prose omits an error type, panics internally on an unreachable channel failure rather
//! than propagating a `RegionUnavailable`-shaped error — this harness's own two dedicated
//! OS threads are never expected to die mid-test (unlike `HardcodedWorld`'s own real
//! production concern, Context: "RegionUnavailable... this project's single hardcoded
//! region has no supervision or restart"), so the additional error-plumbing
//! `HardcodedWorld` needs for that concern is not reproduced here — the established,
//! already-landed convention this harness actually mirrors is `HardcodedWorld::
//! debug_spawn_entity`/`debug_query_block`'s own `pub async fn ... -> T` (or
//! `Option<T>`) shape, not the blueprint's own further-abbreviated non-`async` sketch.
//! This harness also never streams chunks dynamically, never persists players, and never
//! processes block actions/mining — genuinely out of scope for proving ARCH-D10's
//! transfer mechanism and MECH-D19/D21's cross-chunk hopper collapse, this blueprint's
//! own two acceptance criteria.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use bevy_ecs::prelude::*;
use rc_core::{BlockPos, ChunkKey, DimensionId, RcEntityId, RcEntityIdAllocator};
use rc_mechanics::border::RegionOwnership;
use rc_mechanics::entity::physics::ecs::DimensionResource;
use rc_mechanics::entity::transfer::ecs::register_mob_crossing_detection;
use rc_mechanics::entity::{
    BaseEntity, CowBundle, EntityIdentity, EntityKind, EntityPayload, EntityUuid, ItemBundle,
    LivingEntity, MobMarker, NetworkEntityIdAllocator, Pose, SharedNetworkEntityIdAllocator,
    VillagerBundle, ZombieBundle, default_mob_marker,
};
use rc_messaging::{Address, RegionId, RegionMessage};
use rc_physics::{BlockPhysicsProperties, BlockShapeSource, Vec3};
use rc_protocol::{RcPacket, VarInt, encode_payload};
use rc_registries::generated_v776::registries::item;
use rc_scheduler::pool::{RcWorkerPool, SystemTickWaiter, TickClock};
use rc_scheduler::{
    DomainGroup, RcExecutor, RcExecutorBuilder, RegionMessageOutbox, SystemFactory,
};
use rc_transport_inproc::{InProcessTransport, InProcessTransportConfig};
use tokio::sync::{mpsc, oneshot};

use super::block_action::PendingBlockAction;
use super::chunk::{
    PLACEHOLDER_BIOME_ID, SECTION_COUNT, build_placeholder_heightmaps, build_placeholder_light,
    encode_section,
};
use super::connection::PlayerProfile;
use super::keepalive::{KeepAliveAction, KeepAliveDriver};
use super::movement::{
    PendingMoveReport, PendingMovementPacket, PlayerMotion, TeleportState, evaluate_movement,
    feet_block_pos, merge_move_report,
};
use super::packets::{
    ChunkBatchFinished, ChunkBatchStart, GameEvent, KeepAliveClientbound, KeepAliveServerbound,
    LevelChunkWithLight, LoginPlay, SetChunkCacheCenter, SetDefaultSpawnPosition, SetHealth,
    SetPlayerPosition, SynchronizePlayerPosition, pack_position,
};
use super::player_transfer::{
    PlayerConnectionState, PlayerRouting, PlayerRoutingRedirectTable, RegionQueueHandles,
    build_player_entity_snapshot, combined_arrival_driver,
};
use super::world::{PendingJoin, PlayerMarker};
use crate::net::ConnectionHandle;

pub const REGION_WEST_ID: RegionId = RegionId(101);
pub const REGION_EAST_ID: RegionId = RegionId(102);
/// Chunks with `chunk_x < BOUNDARY_CHUNK_X` are owned by West; `>= BOUNDARY_CHUNK_X` by
/// East (Context, Part 1.1's own narrowed `RegionOwnership` contract: both directions
/// resolve to `Address::Region`, never `Address::Chunk`).
pub const BOUNDARY_CHUNK_X: i32 = 0;
/// The full chunk strip both regions' superflat placeholder content spans:
/// `cx in -2..=1, cz in -1..=1` (12 chunks total, 6 per region) — wide enough for a
/// player to walk from deep West territory to deep East territory and back.
pub const STRIP_CHUNK_X_RANGE: std::ops::RangeInclusive<i32> = -2..=1;
pub const STRIP_CHUNK_Z_RANGE: std::ops::RangeInclusive<i32> = -1..=1;

/// A hand-authored placeholder — real vanilla's registry generation does not capture the
/// special-cased `minecraft:player` entity type at all (it is never a data-driven,
/// spawn-egg-style entry the codegen's own `entity_type` registry module carries any
/// constant for), so this harness's own bounded player-tracking stand-in (module doc
/// comment, and `player_to_player_tracking_step` below) picks an arbitrary, never-vanilla-
/// asserted value — no test in this crate ever inspects this value.
const PLAYER_ENTITY_TYPE_STAND_IN: i32 = -1;

/// This harness's own bounded, TwoRegionWorld-local player-visibility range (blocks) —
/// **not** M4-B01's own `compute_tracking_delta`/`EntityKind::client_tracking_range_
/// blocks` mechanism, which is structurally player-blind (`EntityKind` has no `Player`
/// variant, and extending it is out of this blueprint's own scope — a planning decision,
/// never implementation's to make). Chosen so the acceptance test's own player-walk
/// scenario (A crossing from deep West to deep East, B fixed deep in East territory)
/// observes a genuine not-tracked -> tracked transition partway through the walk, never
/// tracked from the very first tick.
const PLAYER_TRACKING_RANGE_BLOCKS: f64 = 20.0;

fn bootstrap_two_region(world: &mut World) {
    world.register_component::<EntityIdentity>();
    world.register_component::<BaseEntity>();
    world.register_component::<LivingEntity>();
    world.register_component::<EntityPayload>();
    world.register_component::<MobMarker>();
    world.register_component::<PlayerMarker>();
    world.register_component::<PlayerMotion>();
    world.register_component::<TeleportState>();
}

/// The destination region's own inbound-queue handles, resource-mirrored into the
/// *source* region's `World` so `system_player_crossing_detection` can redirect a
/// crossing player's own `PlayerRouting` without needing to reach across region/thread
/// boundaries any other way.
#[derive(Resource, Clone)]
struct OtherRegionQueues(RegionQueueHandles);

fn player_crossing_factory() -> SystemFactory {
    Box::new(|| {
        Box::new(IntoSystem::into_system(system_player_crossing_detection))
            as Box<dyn System<In = (), Out = ()>>
    })
}

/// The player-side crossing-detection system (Context, Part 1: "a real
/// crossing-detection system... for players (`rusty-clanker-server`, since `PlayerMarker`/
/// `PlayerMotion` are server-only types `rc-mechanics` must never depend on)"). Mirrors
/// `rc_mechanics::entity::transfer::ecs::system_mob_crossing_detection` exactly, over
/// `PlayerMarker`/`PlayerMotion` instead of `EntityIdentity`/`BaseEntity`.
fn system_player_crossing_detection(
    query: Query<(Entity, &PlayerMarker, &PlayerMotion)>,
    ownership: Res<RegionOwnership>,
    other_queues: Res<OtherRegionQueues>,
    mut outbox: ResMut<RegionMessageOutbox>,
    mut commands: Commands,
) {
    for (entity, marker, motion) in query.iter() {
        let pos = [motion.position.x, motion.position.y, motion.position.z];
        let chunk = feet_block_pos(pos).chunk_key(DimensionId::OVERWORLD);
        let owner = (ownership.resolve)(chunk);
        if owner == ownership.local {
            continue;
        }
        let Address::Region(destination) = owner else {
            continue;
        };

        if let Some(routing) = &marker.routing {
            routing.redirect_to(other_queues.0.clone());
        }

        let payload = super::player_transfer::PlayerTransferPayload {
            uuid: marker.uuid.as_u128(),
            username: marker.username.clone(),
            network_entity_id: marker.network_entity_id,
            position: pos,
            velocity: [motion.velocity.x, motion.velocity.y, motion.velocity.z],
            yaw: motion.yaw,
            pitch: motion.pitch,
            on_ground: motion.on_ground,
            fall_distance: motion.fall_distance,
            tracked_entities: marker.tracked_entities.iter().map(|id| id.0).collect(),
        };
        // No real `RcEntityId` exists for a player anywhere in this project yet
        // (M4-B01's own explicitly-deferred "migrating `PlayerMarker`/`enter_play` onto
        // `BaseEntity`/`LivingEntity`" item) — this outer envelope field is diagnostic-only
        // (M0-B02) and never consulted by the arrival path, which reconstructs identity
        // entirely from the payload's own `uuid`/`network_entity_id`; `entity.to_bits()`
        // is the identical stand-in `entity_tracking.rs`'s own `stand_in_network_id`
        // already establishes for the same structural reason.
        let snapshot = build_player_entity_snapshot(RcEntityId(entity.to_bits()), chunk, &payload);
        outbox.send(
            Address::Region(destination),
            RegionMessage::RegionTransferRequest(Box::new(snapshot)),
        );
        commands.entity(entity).despawn();
    }
}

struct AllAirShapes;
impl BlockShapeSource for AllAirShapes {
    fn properties_at(&self, _pos: BlockPos) -> BlockPhysicsProperties {
        BlockPhysicsProperties::air()
    }
}

fn respond_to_movement_local(
    connection: &ConnectionHandle,
    motion: &PlayerMotion,
    teleport: &TeleportState,
    outcome: super::movement::MovementOutcome,
) {
    use super::movement::MovementOutcome;
    match outcome {
        MovementOutcome::RejectSpeed | MovementOutcome::RejectMismatch => {
            let teleport_id = teleport.awaiting_teleport_id.expect(
                "evaluate_movement always sets awaiting_teleport_id before a Reject* outcome",
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
        MovementOutcome::Disconnect => connection.close(),
        MovementOutcome::NoPositionClaim
        | MovementOutcome::IgnoredAwaitingTeleport
        | MovementOutcome::Accepted => {}
    }
}

fn apply_pending_movement(world: &mut World, pending_moves: &HashMap<i32, PendingMoveReport>) {
    let entries: Vec<(Entity, i32, ConnectionHandle, PlayerMotion, TeleportState)> = {
        let mut query = world.query::<(Entity, &PlayerMarker, &PlayerMotion, &TeleportState)>();
        query
            .iter(world)
            .map(|(entity, marker, motion, teleport)| {
                (
                    entity,
                    marker.network_entity_id,
                    marker.connection.clone(),
                    motion.clone(),
                    teleport.clone(),
                )
            })
            .collect()
    };

    for (entity, network_id, connection, mut motion, mut teleport) in entries {
        let default_report = PendingMoveReport::default();
        let report = pending_moves.get(&network_id).unwrap_or(&default_report);
        let outcome = evaluate_movement(&mut motion, &mut teleport, report, &AllAirShapes);
        if let Some(mut stored) = world.get_mut::<PlayerMotion>(entity) {
            *stored = motion.clone();
        }
        if let Some(mut stored) = world.get_mut::<TeleportState>(entity) {
            *stored = teleport.clone();
        }
        if let Some(mut marker) = world.get_mut::<PlayerMarker>(entity) {
            marker.position = [motion.position.x, motion.position.y, motion.position.z];
            marker.rotation = [motion.yaw, motion.pitch];
            marker.on_ground = motion.on_ground;
        }
        respond_to_movement_local(&connection, &motion, &teleport, outcome);
    }
}

/// This harness's own bounded player-visibility mechanism (module doc comment has the
/// full citation for why this is not M4-B01's own `compute_tracking_delta` mechanism
/// verbatim). Run once per tick, after movement has been resolved for this tick.
fn player_to_player_tracking_step(world: &mut World) {
    let players: Vec<(Entity, i32, u128, [f64; 3], ConnectionHandle)> = {
        let mut query = world.query::<(Entity, &PlayerMarker, &PlayerMotion)>();
        query
            .iter(world)
            .map(|(entity, marker, motion)| {
                (
                    entity,
                    marker.network_entity_id,
                    marker.uuid.as_u128(),
                    [motion.position.x, motion.position.y, motion.position.z],
                    marker.connection.clone(),
                )
            })
            .collect()
    };

    for &(viewer_entity, _viewer_net_id, _viewer_uuid, viewer_pos, ref viewer_conn) in &players {
        for &(other_entity, other_net_id, other_uuid, other_pos, _) in &players {
            if viewer_entity == other_entity {
                continue;
            }
            let dx = other_pos[0] - viewer_pos[0];
            let dy = other_pos[1] - viewer_pos[1];
            let dz = other_pos[2] - viewer_pos[2];
            let in_range = dx * dx + dy * dy + dz * dz
                <= PLAYER_TRACKING_RANGE_BLOCKS * PLAYER_TRACKING_RANGE_BLOCKS;
            let synthetic_id = RcEntityId(other_entity.to_bits());

            let Some(mut marker) = world.get_mut::<PlayerMarker>(viewer_entity) else {
                continue;
            };
            let already_tracked = marker.tracked_entities.contains(&synthetic_id);
            if in_range && !already_tracked {
                marker.tracked_entities.insert(synthetic_id);
                let spawn = super::entity_packets::SpawnEntity {
                    entity_id: other_net_id,
                    uuid: other_uuid,
                    entity_type: PLAYER_ENTITY_TYPE_STAND_IN,
                    x: other_pos[0],
                    y: other_pos[1],
                    z: other_pos[2],
                    movement: super::entity_packets::LpVec3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    pitch: 0,
                    yaw: 0,
                    head_yaw: 0,
                    data: 0,
                };
                let _ = viewer_conn.try_send_payload(encode_payload(&spawn));
            } else if !in_range && already_tracked {
                marker.tracked_entities.remove(&synthetic_id);
                let remove = super::entity_packets::RemoveEntities {
                    entity_ids: vec![VarInt::new(other_net_id)],
                };
                let _ = viewer_conn.try_send_payload(encode_payload(&remove));
            }
        }
    }
}

fn find_player_by_uuid(world: &mut World, uuid: u128) -> Option<[f64; 3]> {
    let mut query = world.query::<&PlayerMarker>();
    query
        .iter(world)
        .find(|marker| marker.uuid.as_u128() == uuid)
        .map(|marker| marker.position)
}

fn spawn_debug_mob(
    world: &mut World,
    rc_entity_ids: &RcEntityIdAllocator,
    network_ids: &NetworkEntityIdAllocator,
    kind: EntityKind,
    pos: BlockPos,
) -> RcEntityId {
    let rc_id = rc_entity_ids.alloc();
    let network_id = network_ids.alloc();
    let identity = EntityIdentity {
        rc_entity_id: rc_id,
        network_entity_id: network_id,
        kind,
    };
    let base = BaseEntity {
        pos: [pos.x as f64, pos.y as f64, pos.z as f64],
        velocity: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0],
        fall_distance: 0.0,
        fire_ticks: 0,
        status_flags: 0,
        air_ticks: 300,
        on_ground: true,
        invulnerable: false,
        portal_cooldown: 0,
        uuid: EntityUuid::new_random(),
        custom_name: None,
        custom_name_visible: false,
        silent: false,
        no_gravity: false,
        glowing: false,
        pose: Pose::Standing,
        ticks_frozen: 0,
        has_visual_fire: false,
    };
    let living = LivingEntity {
        hand_states: 0,
        health: 20.0,
        arrow_count: 0,
        stinger_count: 0,
        sleeping_bed_pos: None,
    };
    let payload = match kind {
        EntityKind::Item => EntityPayload::Item(ItemBundle {
            item: rc_mechanics::entity::ItemStackRecord {
                item_id: item::STONE,
                count: 1,
                components: None,
            },
            pickup_delay_ticks: 0,
            age_ticks: 0,
        }),
        EntityKind::Zombie => EntityPayload::Zombie(ZombieBundle),
        EntityKind::Villager => EntityPayload::Villager(VillagerBundle {
            villager_data: rc_mechanics::entity::metadata::VillagerData {
                villager_type: rc_registries::generated_v776::registries::villager_type::PLAINS,
                profession: rc_registries::generated_v776::registries::villager_profession::NONE,
                level: 1,
            },
        }),
        EntityKind::Cow => EntityPayload::Cow(CowBundle),
    };

    let mut entity_mut = world.spawn((identity, base, payload));
    if kind != EntityKind::Item {
        entity_mut.insert(living);
    }
    if let Some(marker) = default_mob_marker(kind) {
        entity_mut.insert(marker);
    }
    rc_id
}

fn move_debug_mob(world: &mut World, id: RcEntityId, new_pos: BlockPos) -> bool {
    let mut query = world.query::<(&EntityIdentity, &mut BaseEntity)>();
    for (identity, mut base) in query.iter_mut(world) {
        if identity.rc_entity_id == id {
            base.pos = [new_pos.x as f64, new_pos.y as f64, new_pos.z as f64];
            return true;
        }
    }
    false
}

fn query_debug_mob(world: &mut World, id: RcEntityId) -> Option<[f64; 3]> {
    let mut query = world.query::<(&EntityIdentity, &BaseEntity)>();
    query
        .iter(world)
        .find(|(identity, _)| identity.rc_entity_id == id)
        .map(|(_, base)| base.pos)
}

fn build_air_chunk_data() -> Vec<u8> {
    let air_ids = [rc_registries::generated_v776::block_states::default_state::AIR.0; 4096];
    let biome_ids = [PLACEHOLDER_BIOME_ID; 64];
    let mut data = Vec::new();
    for _ in 0..SECTION_COUNT {
        data.extend(encode_section(&air_ids, &biome_ids));
    }
    data
}

struct RegionHandles {
    join_tx: mpsc::UnboundedSender<(PendingJoin, oneshot::Sender<()>)>,
    movement_tx: mpsc::UnboundedSender<PendingMovementPacket>,
    block_action_tx: mpsc::UnboundedSender<PendingBlockAction>,
    query_position_tx: mpsc::UnboundedSender<(u128, oneshot::Sender<Option<[f64; 3]>>)>,
    debug_spawn_mob_tx: mpsc::UnboundedSender<(EntityKind, BlockPos, oneshot::Sender<RcEntityId>)>,
    debug_move_mob_tx: mpsc::UnboundedSender<(RcEntityId, BlockPos, oneshot::Sender<bool>)>,
    debug_query_mob_tx: mpsc::UnboundedSender<(RcEntityId, oneshot::Sender<Option<[f64; 3]>>)>,
}

impl RegionHandles {
    fn queue_handles(&self) -> RegionQueueHandles {
        RegionQueueHandles {
            block_action_tx: self.block_action_tx.clone(),
            movement_tx: self.movement_tx.clone(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_region_tick_loop(
    id: RegionId,
    executor: Arc<RcExecutor>,
    transport: Arc<InProcessTransport>,
    ownership: RegionOwnership,
    other_queues: RegionQueueHandles,
    network_ids: Arc<NetworkEntityIdAllocator>,
    rc_entity_ids: Arc<RcEntityIdAllocator>,
    routing_table: PlayerRoutingRedirectTable,
    shutdown_flag: Arc<AtomicBool>,
    mut join_rx: mpsc::UnboundedReceiver<(PendingJoin, oneshot::Sender<()>)>,
    mut movement_rx: mpsc::UnboundedReceiver<PendingMovementPacket>,
    mut query_position_rx: mpsc::UnboundedReceiver<(u128, oneshot::Sender<Option<[f64; 3]>>)>,
    mut debug_spawn_mob_rx: mpsc::UnboundedReceiver<(
        EntityKind,
        BlockPos,
        oneshot::Sender<RcEntityId>,
    )>,
    mut debug_move_mob_rx: mpsc::UnboundedReceiver<(RcEntityId, BlockPos, oneshot::Sender<bool>)>,
    mut debug_query_mob_rx: mpsc::UnboundedReceiver<(
        RcEntityId,
        oneshot::Sender<Option<[f64; 3]>>,
    )>,
) {
    let mut region = executor.spawn_region(id);
    region.world.insert_resource(ownership);
    region
        .world
        .insert_resource(DimensionResource(DimensionId::OVERWORLD));
    region
        .world
        .insert_resource(SharedNetworkEntityIdAllocator(Arc::clone(&network_ids)));
    region
        .world
        .insert_resource(OtherRegionQueues(other_queues));
    region.world.insert_resource(routing_table.clone());

    let pool = RcWorkerPool::new(2);
    let mut clock = TickClock::<SystemTickWaiter>::new();

    loop {
        if shutdown_flag.load(Ordering::Relaxed) {
            return;
        }

        while let Ok((join, ack)) = join_rx.try_recv() {
            let join_chunk = feet_block_pos(join.position).chunk_key(DimensionId::OVERWORLD);
            let routing = routing_table
                .get(join.uuid.as_u128())
                .map(|state| state.routing);
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
                    sent_chunks: HashSet::new(),
                    tracked_entities: HashSet::new(),
                    last_sent_entity_state: HashMap::new(),
                    routing,
                },
                PlayerMotion {
                    position: Vec3::new(join.position[0], join.position[1], join.position[2]),
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
            ));
            let _ = ack.send(());
        }

        let mut pending_moves: HashMap<i32, PendingMoveReport> = HashMap::new();
        while let Ok(packet) = movement_rx.try_recv() {
            merge_move_report(
                pending_moves.entry(packet.network_entity_id).or_default(),
                &packet.report,
            );
        }
        apply_pending_movement(&mut region.world, &pending_moves);
        player_to_player_tracking_step(&mut region.world);

        while let Ok((kind, pos, reply)) = debug_spawn_mob_rx.try_recv() {
            let id = spawn_debug_mob(&mut region.world, &rc_entity_ids, &network_ids, kind, pos);
            let _ = reply.send(id);
        }
        while let Ok((id, new_pos, reply)) = debug_move_mob_rx.try_recv() {
            let moved = move_debug_mob(&mut region.world, id, new_pos);
            let _ = reply.send(moved);
        }
        while let Ok((id, reply)) = debug_query_mob_rx.try_recv() {
            let pos = query_debug_mob(&mut region.world, id);
            let _ = reply.send(pos);
        }
        while let Ok((uuid, reply)) = query_position_rx.try_recv() {
            let pos = find_player_by_uuid(&mut region.world, uuid);
            let _ = reply.send(pos);
        }

        executor.tick_region(&mut region, &pool, transport.as_ref());

        clock.await_next_tick();
    }
}

pub struct TwoRegionWorld {
    west: RegionHandles,
    east: RegionHandles,
    routing_table: PlayerRoutingRedirectTable,
    network_ids: Arc<NetworkEntityIdAllocator>,
    shutdown_flag: Arc<AtomicBool>,
    threads: std::sync::Mutex<Vec<std::thread::JoinHandle<()>>>,
}

impl TwoRegionWorld {
    /// Spawns both regions' dedicated OS threads (mirroring `HardcodedWorld::new`'s own
    /// established shape, doubled), registers both region ids with one shared
    /// `InProcessTransport`, builds one `RcExecutor` (`register_mob_crossing_detection`
    /// plus this file's own player crossing-detection system, both into
    /// `DomainGroup::EntityPhysicsIntegration`; `with_entity_arrival_driver`
    /// (`player_transfer::combined_arrival_driver`)), inserts `RegionOwnership`/
    /// `SharedNetworkEntityIdAllocator` into both regions' `World`s.
    pub fn new() -> Self {
        let mut builder = RcExecutorBuilder::new(bootstrap_two_region);
        register_mob_crossing_detection(&mut builder);
        builder.register_system(
            DomainGroup::EntityPhysicsIntegration,
            player_crossing_factory(),
            vec![],
        );
        builder.with_entity_arrival_driver(combined_arrival_driver);
        let executor = Arc::new(
            builder
                .build()
                .expect("TwoRegionWorld's own registrations never violate ARCH-D8"),
        );

        let transport = Arc::new(InProcessTransport::new(InProcessTransportConfig::default()));
        transport.register_region(REGION_WEST_ID);
        transport.register_region(REGION_EAST_ID);

        let network_ids = Arc::new(NetworkEntityIdAllocator::new());
        let rc_entity_ids = Arc::new(RcEntityIdAllocator::new());
        let routing_table = PlayerRoutingRedirectTable::default();
        let shutdown_flag = Arc::new(AtomicBool::new(false));

        let (join_tx_w, join_rx_w) = mpsc::unbounded_channel();
        let (movement_tx_w, movement_rx_w) = mpsc::unbounded_channel();
        let (block_action_tx_w, _block_action_rx_w) = mpsc::unbounded_channel();
        let (query_position_tx_w, query_position_rx_w) = mpsc::unbounded_channel();
        let (debug_spawn_mob_tx_w, debug_spawn_mob_rx_w) = mpsc::unbounded_channel();
        let (debug_move_mob_tx_w, debug_move_mob_rx_w) = mpsc::unbounded_channel();
        let (debug_query_mob_tx_w, debug_query_mob_rx_w) = mpsc::unbounded_channel();

        let (join_tx_e, join_rx_e) = mpsc::unbounded_channel();
        let (movement_tx_e, movement_rx_e) = mpsc::unbounded_channel();
        let (block_action_tx_e, _block_action_rx_e) = mpsc::unbounded_channel();
        let (query_position_tx_e, query_position_rx_e) = mpsc::unbounded_channel();
        let (debug_spawn_mob_tx_e, debug_spawn_mob_rx_e) = mpsc::unbounded_channel();
        let (debug_move_mob_tx_e, debug_move_mob_rx_e) = mpsc::unbounded_channel();
        let (debug_query_mob_tx_e, debug_query_mob_rx_e) = mpsc::unbounded_channel();

        let west = RegionHandles {
            join_tx: join_tx_w,
            movement_tx: movement_tx_w,
            block_action_tx: block_action_tx_w,
            query_position_tx: query_position_tx_w,
            debug_spawn_mob_tx: debug_spawn_mob_tx_w,
            debug_move_mob_tx: debug_move_mob_tx_w,
            debug_query_mob_tx: debug_query_mob_tx_w,
        };
        let east = RegionHandles {
            join_tx: join_tx_e,
            movement_tx: movement_tx_e,
            block_action_tx: block_action_tx_e,
            query_position_tx: query_position_tx_e,
            debug_spawn_mob_tx: debug_spawn_mob_tx_e,
            debug_move_mob_tx: debug_move_mob_tx_e,
            debug_query_mob_tx: debug_query_mob_tx_e,
        };

        let west_queues = west.queue_handles();
        let east_queues = east.queue_handles();

        let ownership_west = RegionOwnership {
            local: Address::Region(REGION_WEST_ID),
            resolve: Box::new(|chunk: ChunkKey| {
                if chunk.x < BOUNDARY_CHUNK_X {
                    Address::Region(REGION_WEST_ID)
                } else {
                    Address::Region(REGION_EAST_ID)
                }
            }),
        };
        let ownership_east = RegionOwnership {
            local: Address::Region(REGION_EAST_ID),
            resolve: Box::new(|chunk: ChunkKey| {
                if chunk.x < BOUNDARY_CHUNK_X {
                    Address::Region(REGION_WEST_ID)
                } else {
                    Address::Region(REGION_EAST_ID)
                }
            }),
        };

        let west_thread = {
            let executor = Arc::clone(&executor);
            let transport = Arc::clone(&transport);
            let network_ids = Arc::clone(&network_ids);
            let rc_entity_ids = Arc::clone(&rc_entity_ids);
            let routing_table = routing_table.clone();
            let shutdown_flag = Arc::clone(&shutdown_flag);
            std::thread::spawn(move || {
                run_region_tick_loop(
                    REGION_WEST_ID,
                    executor,
                    transport,
                    ownership_west,
                    east_queues,
                    network_ids,
                    rc_entity_ids,
                    routing_table,
                    shutdown_flag,
                    join_rx_w,
                    movement_rx_w,
                    query_position_rx_w,
                    debug_spawn_mob_rx_w,
                    debug_move_mob_rx_w,
                    debug_query_mob_rx_w,
                )
            })
        };
        let east_thread = {
            let executor = Arc::clone(&executor);
            let transport = Arc::clone(&transport);
            let network_ids = Arc::clone(&network_ids);
            let rc_entity_ids = Arc::clone(&rc_entity_ids);
            let routing_table = routing_table.clone();
            let shutdown_flag = Arc::clone(&shutdown_flag);
            std::thread::spawn(move || {
                run_region_tick_loop(
                    REGION_EAST_ID,
                    executor,
                    transport,
                    ownership_east,
                    west_queues,
                    network_ids,
                    rc_entity_ids,
                    routing_table,
                    shutdown_flag,
                    join_rx_e,
                    movement_rx_e,
                    query_position_rx_e,
                    debug_spawn_mob_rx_e,
                    debug_move_mob_rx_e,
                    debug_query_mob_rx_e,
                )
            })
        };

        Self {
            west,
            east,
            routing_table,
            network_ids,
            shutdown_flag,
            threads: std::sync::Mutex::new(vec![west_thread, east_thread]),
        }
    }

    fn region_for(&self, spawn_pos: BlockPos) -> &RegionHandles {
        if spawn_pos.chunk_x() < BOUNDARY_CHUNK_X {
            &self.west
        } else {
            &self.east
        }
    }

    /// Allocates the next process-wide-unique network entity id (Context, Part 1.6),
    /// shared across every mob/item/player this harness spawns in either region.
    pub fn alloc_network_entity_id(&self) -> i32 {
        self.network_ids.alloc()
    }

    /// Player join: decides West or East by `spawn_pos`'s own chunk (Context, Part
    /// 1.1's harness note); constructs and stores this player's own `PlayerRouting`,
    /// initialized to point at the chosen region's queues; sends the initial join
    /// through that region's own queue, mirroring `HardcodedWorld::queue_join`.
    pub async fn queue_join(&self, join: PendingJoin, spawn_pos: BlockPos) {
        let target = self.region_for(spawn_pos);
        let routing = Arc::new(PlayerRouting::new(target.queue_handles()));
        self.routing_table.insert(
            join.uuid.as_u128(),
            PlayerConnectionState {
                connection: join.connection.clone(),
                routing,
            },
        );

        let (ack_tx, ack_rx) = oneshot::channel();
        let _ = target.join_tx.send((join, ack_tx));
        let _ = ack_rx.await;
    }

    /// Test/debug-only (Context, Part 1.5's own required exact position-delta method):
    /// checks both regions in turn, returns the first `(region_id, position)` hit.
    pub async fn debug_query_player_position(&self, uuid: u128) -> Option<(RegionId, [f64; 3])> {
        for (id, handles) in [(REGION_WEST_ID, &self.west), (REGION_EAST_ID, &self.east)] {
            let (reply_tx, reply_rx) = oneshot::channel();
            if handles.query_position_tx.send((uuid, reply_tx)).is_err() {
                continue;
            }
            if let Ok(Some(pos)) = reply_rx.await {
                return Some((id, pos));
            }
        }
        None
    }

    /// Test/debug-only, mirrors `HardcodedWorld::debug_spawn_entity`/`debug_move_entity`
    /// (M4-B01's own established precedent, this blueprint's own concrete signature):
    /// spawns a mob directly by `BlockPos`, in whichever region currently owns that
    /// position.
    pub async fn debug_spawn_mob(&self, kind: EntityKind, pos: BlockPos) -> RcEntityId {
        let target = self.region_for(pos);
        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = target.debug_spawn_mob_tx.send((kind, pos, reply_tx));
        reply_rx
            .await
            .expect("TwoRegionWorld's own tick-loop threads never die mid-test")
    }

    pub async fn debug_move_mob(&self, id: RcEntityId, new_pos: BlockPos) {
        for handles in [&self.west, &self.east] {
            let (reply_tx, reply_rx) = oneshot::channel();
            if handles
                .debug_move_mob_tx
                .send((id, new_pos, reply_tx))
                .is_err()
            {
                continue;
            }
            if reply_rx.await == Ok(true) {
                return;
            }
        }
    }

    /// Test/debug-only: which region (if any) currently holds `id` as a live entity, and
    /// its current `BaseEntity.pos` — the mob-side analog of
    /// `debug_query_player_position`.
    pub async fn debug_query_mob(&self, id: RcEntityId) -> Option<(RegionId, [f64; 3])> {
        for (region_id, handles) in [(REGION_WEST_ID, &self.west), (REGION_EAST_ID, &self.east)] {
            let (reply_tx, reply_rx) = oneshot::channel();
            if handles.debug_query_mob_tx.send((id, reply_tx)).is_err() {
                continue;
            }
            if let Ok(Some(pos)) = reply_rx.await {
                return Some((region_id, pos));
            }
        }
        None
    }

    /// Full Play-entry sequence over a real loopback connection (mirrors `connection::
    /// enter_play`'s own established sequence, minus persistence/mining/held-item state,
    /// which this harness's own scope does not need) — sends `LoginPlay`, `SetDefaultSpawn
    /// Position`, `SynchronizePlayerPosition`, `GameEvent`, `SetHealth`,
    /// `SetChunkCacheCenter`, then all 12 chunks of `STRIP_CHUNK_X_RANGE`/
    /// `STRIP_CHUNK_Z_RANGE` (all-air placeholder content, this file's own `build_air_
    /// chunk_data`) framed by `ChunkBatchStart`/`ChunkBatchFinished`, queues the join, then
    /// drives the keep-alive + inbound movement-dispatch loop for the connection's
    /// remaining lifetime.
    pub async fn join_and_drive(
        &self,
        handle: ConnectionHandle,
        mut inbound: mpsc::Receiver<rc_protocol::RawPacket>,
        profile: PlayerProfile,
        spawn_pos: BlockPos,
    ) {
        use rc_protocol::{ConnectionState, decode_one};

        handle.set_inbound_state(ConnectionState::Play);
        handle.set_outbound_state(ConnectionState::Play);

        let network_entity_id = self.alloc_network_entity_id();
        let uuid = uuid::Uuid::from_u128(profile.uuid);
        let position = [spawn_pos.x as f64, spawn_pos.y as f64, spawn_pos.z as f64];

        let login_play = LoginPlay {
            entity_id: network_entity_id,
            is_hardcore: false,
            dimension_names: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            view_distance: 5,
            simulation_distance: 2,
            reduced_debug_info: false,
            enable_respawn_screen: true,
            do_limited_crafting: false,
            dimension_type: 0,
            dimension_name: "minecraft:overworld".to_string(),
            hashed_seed: 0,
            game_mode: 1,
            previous_game_mode: -1,
            is_debug: false,
            is_flat: true,
            has_death_location: false,
            portal_cooldown: 0,
            sea_level: 63,
            online_mode: false,
            enforces_secure_chat: false,
        };
        if handle
            .try_send_payload(encode_payload(&login_play))
            .is_err()
        {
            return;
        }

        if handle
            .try_send_payload(encode_payload(&SetDefaultSpawnPosition {
                dimension: "minecraft:overworld".to_string(),
                location: pack_position(spawn_pos),
                yaw: 0.0,
                pitch: 0.0,
            }))
            .is_err()
        {
            return;
        }

        if handle
            .try_send_payload(encode_payload(&SynchronizePlayerPosition {
                teleport_id: 1,
                x: position[0],
                y: position[1],
                z: position[2],
                delta_x: 0.0,
                delta_y: 0.0,
                delta_z: 0.0,
                yaw: 0.0,
                pitch: 0.0,
                relative_arguments: 0x00,
            }))
            .is_err()
        {
            return;
        }

        if handle
            .try_send_payload(encode_payload(&GameEvent {
                event: 13,
                value: 0.0,
            }))
            .is_err()
        {
            return;
        }

        if handle
            .try_send_payload(encode_payload(&SetHealth {
                health: 20.0,
                food: 20,
                saturation: 5.0,
            }))
            .is_err()
        {
            return;
        }

        let player_chunk = spawn_pos.chunk_key(DimensionId::OVERWORLD);
        if handle
            .try_send_payload(encode_payload(&SetChunkCacheCenter {
                chunk_x: player_chunk.x,
                chunk_z: player_chunk.z,
            }))
            .is_err()
        {
            return;
        }
        if handle
            .try_send_payload(encode_payload(&ChunkBatchStart {}))
            .is_err()
        {
            return;
        }

        let heightmaps = build_placeholder_heightmaps();
        let (
            sky_light_mask,
            block_light_mask,
            empty_sky_light_mask,
            empty_block_light_mask,
            sky_light_arrays,
            block_light_arrays,
        ) = build_placeholder_light();
        let air_data = build_air_chunk_data();

        let mut chunk_count = 0i32;
        for chunk_x in STRIP_CHUNK_X_RANGE {
            for chunk_z in STRIP_CHUNK_Z_RANGE {
                let level_chunk = LevelChunkWithLight {
                    chunk_x,
                    chunk_z,
                    heightmaps: heightmaps.clone(),
                    data: air_data.clone(),
                    block_entities: Vec::new(),
                    sky_light_mask: sky_light_mask.clone(),
                    block_light_mask: block_light_mask.clone(),
                    empty_sky_light_mask: empty_sky_light_mask.clone(),
                    empty_block_light_mask: empty_block_light_mask.clone(),
                    sky_light_arrays: sky_light_arrays.clone(),
                    block_light_arrays: block_light_arrays.clone(),
                };
                if handle
                    .try_send_payload(encode_payload(&level_chunk))
                    .is_err()
                {
                    return;
                }
                chunk_count += 1;
            }
        }

        self.queue_join(
            PendingJoin {
                network_entity_id,
                username: profile.username.clone(),
                connection: handle.clone(),
                uuid,
                position,
                rotation: [0.0, 0.0],
            },
            spawn_pos,
        )
        .await;

        if handle
            .try_send_payload(encode_payload(&ChunkBatchFinished {
                batch_size: chunk_count,
            }))
            .is_err()
        {
            return;
        }

        let mut keepalive = KeepAliveDriver::new(std::time::Instant::now());
        let mut poll = tokio::time::interval(std::time::Duration::from_secs(1));

        loop {
            tokio::select! {
                _ = poll.tick() => {
                    match keepalive.on_tick(std::time::Instant::now()) {
                        KeepAliveAction::None => {}
                        KeepAliveAction::SendChallenge(id) => {
                            if handle
                                .try_send_payload(encode_payload(&KeepAliveClientbound { id }))
                                .is_err()
                            {
                                return;
                            }
                        }
                        KeepAliveAction::Disconnect(_) => {
                            handle.close();
                            return;
                        }
                    }
                }
                maybe_raw = inbound.recv() => {
                    let Some(raw) = maybe_raw else { return; };
                    let routing = self
                        .routing_table
                        .get(profile.uuid)
                        .map(|state| state.routing);
                    match raw.id {
                        KeepAliveServerbound::ID => {
                            if let Ok(packet) = decode_one::<KeepAliveServerbound>(raw.body) {
                                let _ = keepalive.on_client_response(packet.id);
                            }
                        }
                        SetPlayerPosition::ID => {
                            if let Ok(packet) = decode_one::<SetPlayerPosition>(raw.body) {
                                let movement_tx = routing
                                    .map(|r| r.current().movement_tx)
                                    .unwrap_or_else(|| self.region_for(spawn_pos).movement_tx.clone());
                                let _ = movement_tx.send(PendingMovementPacket {
                                    network_entity_id,
                                    report: PendingMoveReport {
                                        position: Some(rc_physics::Vec3::new(
                                            packet.x, packet.y, packet.z,
                                        )),
                                        on_ground: Some(packet.on_ground),
                                        ..Default::default()
                                    },
                                });
                            }
                        }
                        other => {
                            tracing::trace!(id = other, "TwoRegionWorld: dropping unrecognized packet");
                        }
                    }
                }
            }
        }
    }
}

impl Default for TwoRegionWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TwoRegionWorld {
    fn drop(&mut self) {
        self.shutdown_flag.store(true, Ordering::Relaxed);
        if let Ok(mut threads) = self.threads.lock() {
            for thread in threads.drain(..) {
                let _ = thread.join();
            }
        }
    }
}
