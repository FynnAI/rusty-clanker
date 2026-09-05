//! M4-B08 — the mob-transfer integration test (Acceptance tests,
//! `mob_region_transfer_integration.rs`): two real `RegionState`s, a real `RcExecutor`
//! with `register_mob_crossing_detection` + `mob_arrival_driver` wired; no sockets, no
//! `rusty-clanker-server`.
//!
//! **Deviation from this blueprint's own literal Acceptance-tests prose, documented**:
//! the blueprint's own prose says "two real `RegionState`s, a real `InProcessTransport`".
//! `rc-mechanics` must never depend on `rc-transport-inproc`, even as a *dev*-dependency
//! — `xtask lint-deps`'s Rule 2 (SIM/NETRENDER split) evaluates `cargo metadata`'s own
//! `Node::dependencies` field, which includes every dependency kind (confirmed against
//! `xtask/src/metadata.rs`'s own doc comment: "All resolved dependency edges from this
//! node, any kind"), so a dev-dependency edge from `rc-mechanics` (a `SIM` crate) to
//! `rc-transport-inproc` (a `NETRENDER` crate) would trip Rule 2 exactly like a normal
//! dependency edge would. This file's own local `MockTransport` (below) is a real,
//! actually-routing `Transport` implementation — not a no-op stub — reused from this
//! project's own already-established `crates/scheduler/tests/common/mod.rs` convention
//! (restated here, since a `tests/*.rs` integration-test file cannot itself depend on
//! another crate's own `tests/common` module).
#![cfg(feature = "server-systems")]

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use bevy_ecs::prelude::*;
use rc_core::{ChunkKey, DimensionId, RcEntityId};
use rc_mechanics::border::RegionOwnership;
use rc_mechanics::entity::physics::ecs::DimensionResource;
use rc_mechanics::entity::transfer::ecs::register_mob_crossing_detection;
use rc_mechanics::entity::{
    AiSystemKind, BaseEntity, CowBundle, EntityIdentity, EntityKind, EntityPayload, EntityUuid,
    ItemBundle, ItemStackRecord, LivingEntity, MobMarker, NetworkEntityIdAllocator, Pose,
    SharedNetworkEntityIdAllocator, VillagerBundle, ZombieBundle, default_mob_marker,
    mob_arrival_driver,
};
use rc_messaging::{Address, Message, RegionId, RegionMessage, Transport, TransportError};
use rc_registries::generated_v776::registries::item;
use rc_scheduler::pool::RcWorkerPool;
use rc_scheduler::{RcExecutor, RcExecutorBuilder, RegionState};

/// A real, actually-routing `Transport` double (module doc comment has the full
/// deviation citation) — `send` genuinely enqueues into the destination region's own
/// inbox, unlike `crates/scheduler/tests/common/mod.rs`'s own `MockTransport` (which
/// only records `sent` for observation and requires an explicit `seed` call to make
/// anything `try_recv`-able).
struct MockTransport {
    inboxes: Mutex<HashMap<RegionId, VecDeque<Message<RegionMessage>>>>,
}

impl MockTransport {
    fn new() -> Self {
        Self {
            inboxes: Mutex::new(HashMap::new()),
        }
    }

    /// Non-consuming: how many messages are currently queued for `id`.
    fn queued_len(&self, id: RegionId) -> usize {
        self.inboxes
            .lock()
            .unwrap()
            .get(&id)
            .map(VecDeque::len)
            .unwrap_or(0)
    }
}

impl Transport for MockTransport {
    fn send(&self, msg: Message<RegionMessage>) -> Result<(), TransportError> {
        let Address::Region(to) = msg.to else {
            return Err(TransportError::Backpressure(msg));
        };
        self.inboxes
            .lock()
            .unwrap()
            .entry(to)
            .or_default()
            .push_back(msg);
        Ok(())
    }

    fn try_recv(&self, into: RegionId) -> Option<Message<RegionMessage>> {
        self.inboxes
            .lock()
            .unwrap()
            .get_mut(&into)
            .and_then(VecDeque::pop_front)
    }
}

fn bootstrap(world: &mut World) {
    world.register_component::<EntityIdentity>();
    world.register_component::<BaseEntity>();
    world.register_component::<LivingEntity>();
    world.register_component::<EntityPayload>();
    world.register_component::<MobMarker>();
}

const REGION_1: RegionId = RegionId(1);
const REGION_2: RegionId = RegionId(2);

/// `RegionOwnership` for `RegionId(1)` (`local = Address::Region(RegionId(1))`,
/// `resolve(chunk) = if chunk.x < 0 { Address::Region(RegionId(1)) } else {
/// Address::Region(RegionId(2)) }`) and the mirror for `RegionId(2)` (Acceptance tests'
/// own required fixture).
fn ownership_for(local: RegionId) -> RegionOwnership {
    RegionOwnership {
        local: Address::Region(local),
        resolve: Box::new(|chunk: ChunkKey| {
            if chunk.x < 0 {
                Address::Region(REGION_1)
            } else {
                Address::Region(REGION_2)
            }
        }),
    }
}

fn build_two_region_fixture() -> (RcExecutor, RegionState, RegionState) {
    let mut builder = RcExecutorBuilder::new(bootstrap);
    register_mob_crossing_detection(&mut builder);
    builder.with_entity_arrival_driver(mob_arrival_driver);
    let executor = builder.build().expect("build should succeed");

    let mut region1 = executor.spawn_region(REGION_1);
    let mut region2 = executor.spawn_region(REGION_2);

    region1
        .world
        .insert_resource(DimensionResource(DimensionId::OVERWORLD));
    region2
        .world
        .insert_resource(DimensionResource(DimensionId::OVERWORLD));
    region1.world.insert_resource(ownership_for(REGION_1));
    region2.world.insert_resource(ownership_for(REGION_2));

    (executor, region1, region2)
}

fn sample_base(pos: [f64; 3]) -> BaseEntity {
    BaseEntity {
        pos,
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
    }
}

fn sample_living() -> LivingEntity {
    LivingEntity {
        hand_states: 0,
        health: 20.0,
        arrow_count: 0,
        stinger_count: 0,
        sleeping_bed_pos: None,
    }
}

fn sample_payload_for(kind: EntityKind) -> (Option<LivingEntity>, EntityPayload) {
    match kind {
        EntityKind::Item => (
            None,
            EntityPayload::Item(ItemBundle {
                item: ItemStackRecord {
                    item_id: item::STONE,
                    count: 1,
                    components: None,
                },
                pickup_delay_ticks: 0,
                age_ticks: 0,
            }),
        ),
        EntityKind::Zombie => (Some(sample_living()), EntityPayload::Zombie(ZombieBundle)),
        EntityKind::Villager => (
            Some(sample_living()),
            EntityPayload::Villager(VillagerBundle {
                villager_data: rc_mechanics::entity::metadata::VillagerData {
                    villager_type: rc_registries::generated_v776::registries::villager_type::PLAINS,
                    profession:
                        rc_registries::generated_v776::registries::villager_profession::NONE,
                    level: 1,
                },
            }),
        ),
        EntityKind::Cow => (Some(sample_living()), EntityPayload::Cow(CowBundle)),
    }
}

fn spawn_mob(
    world: &mut World,
    rc_id: u64,
    network_id: i32,
    kind: EntityKind,
    pos: [f64; 3],
) -> Entity {
    let identity = EntityIdentity {
        rc_entity_id: RcEntityId(rc_id),
        network_entity_id: network_id,
        kind,
    };
    let base = sample_base(pos);
    let (living, payload) = sample_payload_for(kind);

    let mut entity_mut = world.spawn((identity, base, payload));
    if let Some(living) = living {
        entity_mut.insert(living);
    }
    if let Some(marker) = default_mob_marker(kind) {
        entity_mut.insert(marker);
    }
    entity_mut.id()
}

fn find_by_rc_id(world: &mut World, rc_id: u64) -> Option<(Entity, EntityIdentity)> {
    let mut query = world.query::<(Entity, &EntityIdentity)>();
    query
        .iter(world)
        .find(|(_, identity)| identity.rc_entity_id == RcEntityId(rc_id))
        .map(|(entity, identity)| (entity, *identity))
}

#[test]
fn mob_crossing_west_to_east_arrives_exactly_one_tick_later() {
    let (executor, mut region1, mut region2) = build_two_region_fixture();
    let transport = MockTransport::new();
    let pool = RcWorkerPool::new(1);

    let entity = spawn_mob(
        &mut region1.world,
        1,
        5,
        EntityKind::Zombie,
        [-2.0, 64.0, 0.0],
    );
    region1.world.get_mut::<BaseEntity>(entity).unwrap().pos = [2.0, 64.0, 0.0];

    executor.tick_region(&mut region1, &pool, &transport);

    assert!(
        find_by_rc_id(&mut region1.world, 1).is_none(),
        "the entity must no longer be present in region 1's World"
    );
    assert_eq!(
        transport.queued_len(REGION_2),
        1,
        "exactly one RegionTransferRequest must be queued for region 2"
    );

    executor.tick_region(&mut region2, &pool, &transport);

    let (arrived, identity) =
        find_by_rc_id(&mut region2.world, 1).expect("the entity must now be present in region 2");
    assert_eq!(identity.network_entity_id, 5);
    assert_eq!(identity.kind, EntityKind::Zombie);
    let base = region2.world.get::<BaseEntity>(arrived).unwrap();
    assert_eq!(base.pos, [2.0, 64.0, 0.0]);
}

#[test]
fn ai_system_kind_is_reconstructed_identically_across_transfer() {
    let (executor, mut region1, mut region2) = build_two_region_fixture();
    let transport = MockTransport::new();
    let pool = RcWorkerPool::new(1);

    let entity = spawn_mob(
        &mut region1.world,
        2,
        6,
        EntityKind::Villager,
        [-2.0, 64.0, 0.0],
    );
    let marker_before = region1
        .world
        .get::<MobMarker>(entity)
        .cloned()
        .expect("a Villager must carry a MobMarker");
    assert_eq!(marker_before.ai_system, AiSystemKind::Brain);

    region1.world.get_mut::<BaseEntity>(entity).unwrap().pos = [2.0, 64.0, 0.0];
    executor.tick_region(&mut region1, &pool, &transport);
    executor.tick_region(&mut region2, &pool, &transport);

    let (arrived, identity) =
        find_by_rc_id(&mut region2.world, 2).expect("must have arrived in region 2");
    assert_eq!(identity.kind, EntityKind::Villager);
    let marker_after = region2
        .world
        .get::<MobMarker>(arrived)
        .cloned()
        .expect("MobMarker must be reconstructed on arrival");
    assert_eq!(marker_after.ai_system, AiSystemKind::Brain);
}

#[test]
fn network_entity_id_never_collides_and_never_changes() {
    let (executor, mut region1, mut region2) = build_two_region_fixture();
    let allocator = Arc::new(NetworkEntityIdAllocator::new());
    region1
        .world
        .insert_resource(SharedNetworkEntityIdAllocator(Arc::clone(&allocator)));
    region2
        .world
        .insert_resource(SharedNetworkEntityIdAllocator(Arc::clone(&allocator)));

    let n1 = allocator.alloc();
    let n2 = allocator.alloc();
    assert_ne!(
        n1, n2,
        "the shared allocator must never hand out the same id twice"
    );

    let transport = MockTransport::new();
    let pool = RcWorkerPool::new(1);

    let entity1 = spawn_mob(
        &mut region1.world,
        10,
        n1,
        EntityKind::Cow,
        [-2.0, 64.0, 0.0],
    );
    let _entity2 = spawn_mob(
        &mut region2.world,
        11,
        n2,
        EntityKind::Cow,
        [2.0, 64.0, 0.0],
    );

    region1.world.get_mut::<BaseEntity>(entity1).unwrap().pos = [2.0, 64.0, 0.0];
    executor.tick_region(&mut region1, &pool, &transport);
    executor.tick_region(&mut region2, &pool, &transport);

    let (_, identity) =
        find_by_rc_id(&mut region2.world, 10).expect("must have arrived in region 2");
    assert_eq!(identity.network_entity_id, n1);
    assert_ne!(identity.network_entity_id, n2);
}

#[test]
fn re_crossing_within_consecutive_ticks_causes_no_data_loss_or_duplication() {
    let (executor, mut region1, mut region2) = build_two_region_fixture();
    let transport = MockTransport::new();
    let pool = RcWorkerPool::new(1);

    let entity = spawn_mob(&mut region1.world, 1, 9, EntityKind::Cow, [-2.0, 64.0, 0.0]);
    region1.world.get_mut::<BaseEntity>(entity).unwrap().pos = [2.0, 64.0, 0.0];

    for round in 0..3 {
        // West -> East.
        executor.tick_region(&mut region1, &pool, &transport);
        assert!(
            find_by_rc_id(&mut region1.world, 1).is_none(),
            "round {round}: must not remain in region 1 after crossing east"
        );
        executor.tick_region(&mut region2, &pool, &transport);
        let (arrived_east, identity) = find_by_rc_id(&mut region2.world, 1)
            .unwrap_or_else(|| panic!("round {round}: must have arrived in region 2"));
        assert_eq!(identity.rc_entity_id, RcEntityId(1));
        assert_eq!(identity.network_entity_id, 9);
        assert!(find_by_rc_id(&mut region1.world, 1).is_none());

        // East -> West.
        region2
            .world
            .get_mut::<BaseEntity>(arrived_east)
            .unwrap()
            .pos = [-2.0, 64.0, 0.0];
        executor.tick_region(&mut region2, &pool, &transport);
        assert!(
            find_by_rc_id(&mut region2.world, 1).is_none(),
            "round {round}: must not remain in region 2 after crossing west"
        );
        executor.tick_region(&mut region1, &pool, &transport);
        let (arrived_west, identity) = find_by_rc_id(&mut region1.world, 1)
            .unwrap_or_else(|| panic!("round {round}: must have arrived back in region 1"));
        assert_eq!(identity.rc_entity_id, RcEntityId(1));
        assert_eq!(identity.network_entity_id, 9);
        assert!(find_by_rc_id(&mut region2.world, 1).is_none());

        // Set up the next round trip.
        region1
            .world
            .get_mut::<BaseEntity>(arrived_west)
            .unwrap()
            .pos = [2.0, 64.0, 0.0];
    }
}

#[test]
fn entity_absent_from_both_worlds_during_the_in_flight_tick() {
    let (executor, mut region1, mut region2) = build_two_region_fixture();
    let transport = MockTransport::new();
    let pool = RcWorkerPool::new(1);

    let entity = spawn_mob(
        &mut region1.world,
        1,
        5,
        EntityKind::Zombie,
        [-2.0, 64.0, 0.0],
    );
    region1.world.get_mut::<BaseEntity>(entity).unwrap().pos = [2.0, 64.0, 0.0];

    executor.tick_region(&mut region1, &pool, &transport);

    assert!(
        find_by_rc_id(&mut region1.world, 1).is_none(),
        "must be absent from region 1 during the in-flight tick"
    );
    assert!(
        find_by_rc_id(&mut region2.world, 1).is_none(),
        "must be absent from region 2 during the in-flight tick (it has not yet ticked)"
    );

    executor.tick_region(&mut region2, &pool, &transport);

    assert!(find_by_rc_id(&mut region2.world, 1).is_some());
}
