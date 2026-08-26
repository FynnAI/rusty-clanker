//! M2-B07 acceptance test: `apply_block_action`'s cross-region branch (ARCH-D11/D25/D30)
//! against the real message substrate -- no sockets exercising a scripted client, no
//! `HardcodedWorld` (mirroring `M0-B03`'s own `cross_region_timing.rs` `FakeRegion`
//! pattern). The one throwaway `ConnectionHandle` `PendingBlockAction` requires still
//! needs a real loopback socket pair to construct (`net::spawn_connection`'s own
//! signature) -- unrelated to, and never exercising, the wire protocol itself.

use bevy_ecs::prelude::*;
use rc_chunk_storage::{BlockStateColumn, ChunkKeyTag, PaletteThresholds, RegistryId};
use rc_core::{BlockPos, ChunkKey, DimensionId};
use rc_messaging::{
    Address, BorderUpdateEvent, BorderUpdateKind, RegionId, RegionMessage, RegionMessageBus,
    RegionMessageState, Transport,
};
use rc_registries::generated_v776::block_states::default_state::{AIR, BEDROCK, STONE};
use rc_transport_inproc::{InProcessTransport, InProcessTransportConfig};
use rusty_clanker_server::net::{ConnectionConfig, ConnectionHandle, spawn_connection};
use rusty_clanker_server::play::{
    ApplyOutcome, BlockActionKind, ChunkIndex, Face, PendingBlockAction, apply_block_action,
    seed_chunk_column,
};

/// A real, throwaway `ConnectionHandle` -- `PendingBlockAction`'s own field requires one,
/// but this test never sends anything through it.
async fn throwaway_connection_handle() -> ConnectionHandle {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (accept_result, connect_result) =
        tokio::join!(listener.accept(), tokio::net::TcpStream::connect(addr));
    let (server, _) = accept_result.unwrap();
    let _client = connect_result.unwrap();
    let (_inbound, handle) = spawn_connection(server, ConnectionConfig::default());
    handle
}

/// Builds a fresh `World` carrying exactly one locally-owned chunk entity, `(0, 0)`,
/// seeded the same way `HardcodedWorld`'s own bootstrap seeds every chunk (Context: "The
/// chunk-entity gap"), plus a `ChunkIndex` mapping only that one key.
fn world_with_one_local_chunk() -> (World, Entity) {
    let mut world = World::new();
    let (blocks, biomes, light, heightmaps, block_entities, status, persistence) =
        seed_chunk_column(PaletteThresholds::blocks(15), PaletteThresholds::biomes(4));
    let key = ChunkKey::new(DimensionId::OVERWORLD, 0, 0);
    let entity = world
        .spawn((
            ChunkKeyTag(key),
            blocks,
            biomes,
            light,
            heightmaps,
            block_entities,
            status,
            persistence,
        ))
        .id();
    let mut chunk_index = ChunkIndex::default();
    chunk_index.0.insert(key, entity);
    world.insert_resource(chunk_index);
    (world, entity)
}

/// `(0, 0)` is local (`RegionId(1)`, this test's own `local_identity`); every other chunk
/// resolves to `RegionId(2)` -- a neighbor this `World` never holds a chunk entity for
/// (ARCH-D5: "no two regions ever hold a chunk simultaneously").
fn resolve_owner(key: ChunkKey) -> Address {
    if key == ChunkKey::new(DimensionId::OVERWORLD, 0, 0) {
        Address::Region(RegionId(1))
    } else {
        Address::Region(RegionId(2))
    }
}

async fn run_cross_region_case(kind: BlockActionKind, target: BlockPos, expected_new_state: u32) {
    let (mut world, entity) = world_with_one_local_chunk();
    let local_identity = Address::Region(RegionId(1));
    let connection = throwaway_connection_handle().await;
    let action = PendingBlockAction {
        network_entity_id: 1,
        connection,
        kind,
        sequence: 1,
    };

    let mut bus = RegionMessageBus::new();
    let outcome = apply_block_action(
        &mut world,
        DimensionId::OVERWORLD,
        &action,
        &resolve_owner,
        local_identity,
        &mut bus,
    );

    assert_eq!(
        outcome,
        ApplyOutcome::RoutedCrossRegion {
            pos: target,
            new_state: expected_new_state,
        }
    );

    // Chunk (0, 0)'s own `BlockStateColumn` is completely untouched -- no accidental
    // local mutation from a cross-region action.
    let column = world.get::<BlockStateColumn>(entity).unwrap();
    assert_eq!(column.get(5, -64, 5).to_raw(), BEDROCK.0);

    let expected_payload = RegionMessage::BorderUpdateEvent(BorderUpdateEvent {
        chunk: ChunkKey::new(DimensionId::OVERWORLD, 5, 5),
        pos: target,
        kind: BorderUpdateKind::BlockChanged {
            new_state: expected_new_state,
        },
    });

    let mut state = RegionMessageState::new();
    state.merge(bus);
    let outgoing = state.drain_outbox(RegionId(1), 0);
    assert_eq!(outgoing.len(), 1);
    assert_eq!(outgoing[0].payload, expected_payload);

    let transport = InProcessTransport::new(InProcessTransportConfig::default());
    transport.register_region(RegionId(2));
    for msg in outgoing {
        transport.send(msg).unwrap();
    }

    let received = transport.try_recv(RegionId(2));
    assert_eq!(received.map(|m| m.payload), Some(expected_payload));
    assert!(transport.try_recv(RegionId(2)).is_none());
}

#[tokio::test]
async fn cross_region_target_is_forwarded_via_border_update_event_never_mutated_locally() {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        // (85, -60, 85) is chunk (5, 5) -- not (0, 0), and has no entity in this test's own
        // `World` at all.
        let target = BlockPos::new(85, -60, 85);
        run_cross_region_case(BlockActionKind::Break { location: target }, target, AIR.0).await;
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn cross_region_placement_forwards_the_fixed_placement_block() {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        let target = BlockPos::new(85, -59, 85);
        run_cross_region_case(
            BlockActionKind::Place {
                location: target,
                face: Face::Up,
                inside_block: true,
            },
            target,
            STONE.0,
        )
        .await;
    })
    .await
    .unwrap();
}
